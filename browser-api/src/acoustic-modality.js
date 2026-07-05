'use strict';

const CHIRP_F_START = 18000;
const CHIRP_F_END   = 22000;
const CHIRP_DURATION_S = 0.02;
const SPEED_OF_SOUND   = 343;

/**
 * Acoustic FMCW modality — emits an ultrasonic chirp from the speaker and
 * captures reflections on the microphone. Runs entirely in-browser via the
 * WebAudio API. No extension or native code required.
 */
export class AcousticModality {
  /** @param {number} sampleRate */
  constructor(sampleRate = 44100) {
    this._sampleRate = sampleRate;
    this._ctx = null;
    this._stream = null;
    this._analyser = null;
    this._chirpBuffer = null;
    this._active = false;
  }

  async init() {
    this._ctx = new AudioContext({ sampleRate: this._sampleRate });
    this._stream = await navigator.mediaDevices.getUserMedia({ audio: true, video: false });
    this._chirpBuffer = this._buildChirp();

    const source = this._ctx.createMediaStreamSource(this._stream);
    this._analyser = this._ctx.createAnalyser();
    this._analyser.fftSize = 2048;
    source.connect(this._analyser);
  }

  start() {
    if (!this._ctx) throw new Error('AcousticModality.init() not called');
    this._active = true;
    this._scheduleChirp();
  }

  stop() {
    this._active = false;
    this._stream?.getTracks().forEach(t => t.stop());
    this._ctx?.close();
  }

  /**
   * Read the current beat spectrum from the analyser and derive range.
   * @returns {{ rangeM: number, beatSpectrum: Float32Array }}
   */
  readFrame() {
    if (!this._analyser) return { rangeM: null, beatSpectrum: null };

    const bins = this._analyser.frequencyBinCount;
    const spectrum = new Float32Array(bins);
    this._analyser.getFloatFrequencyData(spectrum);

    // Convert dB to linear magnitude
    const linear = spectrum.map(db => Math.pow(10, db / 20));

    // Only look in the beat frequency band: 0 – (chirp bandwidth) Hz
    const maxBeatHz  = CHIRP_F_END - CHIRP_F_START;
    const hzPerBin   = this._sampleRate / (bins * 2);
    const maxBin     = Math.floor(maxBeatHz / hzPerBin);

    let peakBin = 0, peakVal = -Infinity;
    for (let i = 1; i < maxBin; i++) {
      if (linear[i] > peakVal) { peakVal = linear[i]; peakBin = i; }
    }

    const beatHz = peakBin * hzPerBin;
    const chirpRate = (CHIRP_F_END - CHIRP_F_START) / CHIRP_DURATION_S;
    const rangeM = (beatHz * SPEED_OF_SOUND) / (2 * chirpRate);

    return { rangeM, beatSpectrum: linear };
  }

  _buildChirp() {
    const n = Math.floor(CHIRP_DURATION_S * this._sampleRate);
    const buffer = this._ctx.createBuffer(1, n, this._sampleRate);
    const data = buffer.getChannelData(0);
    const chirpRate = (CHIRP_F_END - CHIRP_F_START) / CHIRP_DURATION_S;

    for (let k = 0; k < n; k++) {
      const t = k / this._sampleRate;
      data[k] = Math.sin(2 * Math.PI * (CHIRP_F_START * t + 0.5 * chirpRate * t * t));
    }
    return buffer;
  }

  _scheduleChirp() {
    if (!this._active) return;
    const src = this._ctx.createBufferSource();
    src.buffer = this._chirpBuffer;

    // Low gain so the chirp is inaudible below 16 kHz bleed
    const gain = this._ctx.createGain();
    gain.gain.value = 0.05;
    src.connect(gain);
    gain.connect(this._ctx.destination);

    src.start();
    src.onended = () => {
      if (this._active) setTimeout(() => this._scheduleChirp(), 30);
    };
  }
}
