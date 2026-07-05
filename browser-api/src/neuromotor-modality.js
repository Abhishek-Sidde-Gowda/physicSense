'use strict';

const SAMPLE_RATE = 200; // Hz — neuromotor signals are low-frequency

/**
 * Neuromotor modality — runs entirely in JS without any hardware beyond what
 * the acoustic modality provides. Processes the acoustic phase displacement
 * signal through clinical tremor bands and a gait step detector.
 *
 * In the full system this is backed by the Rust WASM fusion engine. This JS
 * fallback implements the same algorithms for environments where WASM is not
 * yet loaded, ensuring the API always returns valid (if lower-fidelity) data.
 */
export class NeuromotorModality {
  constructor() {
    this._phaseBuffer = new Float32Array(SAMPLE_RATE * 10); // 10 s window
    this._writeHead = 0;
    this._full = false;
  }

  /**
   * Push a new phase displacement sample (metres, from acoustic ranging).
   * @param {number} phaseSample
   */
  push(phaseSample) {
    this._phaseBuffer[this._writeHead] = phaseSample;
    this._writeHead = (this._writeHead + 1) % this._phaseBuffer.length;
    if (this._writeHead === 0) this._full = true;
  }

  /**
   * Compute neuromotor output from the buffered phase signal.
   * @returns {import('./index.d.ts').PhysicSenseNeuromotor}
   */
  compute() {
    const signal = this._full
      ? this._contiguous()
      : this._phaseBuffer.slice(0, this._writeHead);

    if (signal.length < SAMPLE_RATE * 2) {
      return this._emptyResult();
    }

    const tremorDominantHz = this._dominantHz(signal);
    const tremorClass      = this._classifyTremor(signal, tremorDominantHz);
    const { cadenceSpm, asymmetryIndex } = this._gaitMetrics(signal);
    const updrsProxyScore  = this._updrsScore(tremorClass, signal, asymmetryIndex);

    return {
      tremorDominantHz,
      tremorClass,
      gaitCadenceSpm:     cadenceSpm,
      gaitAsymmetryIndex: asymmetryIndex,
      updrsProxyScore,
      flagForReview: updrsProxyScore >= 3.0,
    };
  }

  // --- private ---

  _contiguous() {
    const buf = this._phaseBuffer;
    const h = this._writeHead;
    const out = new Float32Array(buf.length);
    out.set(buf.subarray(h));
    out.set(buf.subarray(0, h), buf.length - h);
    return out;
  }

  _dominantHz(signal) {
    const n = nextPow2(signal.length);
    const re = new Float32Array(n);
    const im = new Float32Array(n);
    signal.forEach((v, i) => { re[i] = v; });
    fftInPlace(re, im, n);

    const minBin = Math.floor(0.5 * n / SAMPLE_RATE);
    const maxBin = Math.floor(20  * n / SAMPLE_RATE);

    let peakBin = minBin, peakMag = 0;
    for (let k = minBin; k < maxBin; k++) {
      const mag = Math.hypot(re[k], im[k]);
      if (mag > peakMag) { peakMag = mag; peakBin = k; }
    }
    if (peakMag < 1e-4) return 0;
    return peakBin * SAMPLE_RATE / n;
  }

  _classifyTremor(signal, hz) {
    if (hz === 0) return 'none';
    const parkPwr  = bandPower(signal, 3,  6,  SAMPLE_RATE);
    const essPwr   = bandPower(signal, 4,  12, SAMPLE_RATE);
    const physPwr  = bandPower(signal, 8,  12, SAMPLE_RATE);
    const total    = parkPwr + essPwr + physPwr + 1e-10;

    if (parkPwr / total > 0.4 && parkPwr >= essPwr) return 'parkinsonian';
    if (essPwr  / total > 0.4 && essPwr  >= physPwr) return 'essential';
    if (physPwr / total > 0.4) return 'physiological';
    if ((parkPwr + essPwr + physPwr) / total > 0.3) return 'indeterminate';
    return 'none';
  }

  _gaitMetrics(signal) {
    const threshold = 0.15;
    const steps = [];
    let inStep = false, peakIdx = 0, peakVal = 0;

    for (let i = 0; i < signal.length; i++) {
      const abs = Math.abs(signal[i]);
      if (abs > threshold) {
        if (!inStep) { inStep = true; peakVal = abs; peakIdx = i; }
        else if (abs > peakVal) { peakVal = abs; peakIdx = i; }
      } else if (inStep) {
        steps.push(peakIdx);
        inStep = false; peakVal = 0;
      }
    }

    if (steps.length < 4) return { cadenceSpm: 0, asymmetryIndex: 0 };

    const intervals = steps.slice(1).map((s, i) => (s - steps[i]) / SAMPLE_RATE);
    const meanInt   = mean(intervals);
    const cadenceSpm = meanInt > 0 ? 60 / meanInt : 0;

    const asymmetryIndex = mean(
      chunk2(intervals).map(([a, b]) => Math.abs(a - b) / ((a + b) || 1e-6))
    );

    return { cadenceSpm, asymmetryIndex };
  }

  _updrsScore(tremorClass, signal, asymmetryIndex) {
    const tremorScore = {
      none: 0, physiological: 0.5, indeterminate: 1,
      essential: 1.5, parkinsonian: 2.5,
    }[tremorClass] ?? 0;

    const cv = coefficientOfVariation(signal);
    const gaitScore = Math.min(asymmetryIndex * 1.5 + (cv / 30) * 1.5, 4);
    return Math.min(tremorScore + gaitScore, 8);
  }

  _emptyResult() {
    return {
      tremorDominantHz: 0, tremorClass: 'none',
      gaitCadenceSpm: 0,   gaitAsymmetryIndex: 0,
      updrsProxyScore: 0,  flagForReview: false,
    };
  }
}

// --- DSP helpers ---

function nextPow2(n) {
  let p = 1;
  while (p < n) p <<= 1;
  return p;
}

/** Cooley-Tukey radix-2 DIT FFT in-place */
function fftInPlace(re, im, n) {
  for (let i = 1, j = 0; i < n; i++) {
    let bit = n >> 1;
    for (; j & bit; bit >>= 1) j ^= bit;
    j ^= bit;
    if (i < j) {
      [re[i], re[j]] = [re[j], re[i]];
      [im[i], im[j]] = [im[j], im[i]];
    }
  }
  for (let len = 2; len <= n; len <<= 1) {
    const ang = (-2 * Math.PI) / len;
    const wRe = Math.cos(ang), wIm = Math.sin(ang);
    for (let i = 0; i < n; i += len) {
      let uRe = 1, uIm = 0;
      for (let j = 0; j < len / 2; j++) {
        const eRe = re[i+j], eIm = im[i+j];
        const oRe = re[i+j+len/2] * uRe - im[i+j+len/2] * uIm;
        const oIm = re[i+j+len/2] * uIm + im[i+j+len/2] * uRe;
        re[i+j]       = eRe + oRe; im[i+j]       = eIm + oIm;
        re[i+j+len/2] = eRe - oRe; im[i+j+len/2] = eIm - oIm;
        const newURe = uRe * wRe - uIm * wIm;
        uIm = uRe * wIm + uIm * wRe; uRe = newURe;
      }
    }
  }
}

function bandPower(signal, fLow, fHigh, sr) {
  const n = nextPow2(signal.length);
  const re = new Float32Array(n);
  const im = new Float32Array(n);
  signal.forEach((v, i) => { re[i] = v; });
  fftInPlace(re, im, n);
  let power = 0;
  const lo = Math.floor(fLow  * n / sr);
  const hi = Math.floor(fHigh * n / sr);
  for (let k = lo; k <= hi && k < n / 2; k++) {
    power += re[k] ** 2 + im[k] ** 2;
  }
  return power / (hi - lo + 1);
}

function mean(arr) {
  return arr.length ? arr.reduce((a, b) => a + b, 0) / arr.length : 0;
}

function coefficientOfVariation(arr) {
  const m = mean(arr);
  if (m < 1e-10) return 0;
  const variance = mean(arr.map(x => (x - m) ** 2));
  return Math.sqrt(variance) / m;
}

function chunk2(arr) {
  const out = [];
  for (let i = 0; i + 1 < arr.length; i += 2) out.push([arr[i], arr[i+1]]);
  return out;
}
