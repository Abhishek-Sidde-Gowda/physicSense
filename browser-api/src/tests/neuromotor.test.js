/**
 * Tests for the JS neuromotor modality (fallback implementation).
 * Run with: node --test src/tests/neuromotor.test.js
 */
import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { NeuromotorModality } from '../neuromotor-modality.js';

const SAMPLE_RATE = 200;

function sineWave(freqHz, n) {
  return Array.from({ length: n }, (_, k) =>
    Math.sin(2 * Math.PI * freqHz * k / SAMPLE_RATE)
  );
}

describe('NeuromotorModality', () => {
  it('returns none for silence', () => {
    const nm = new NeuromotorModality();
    // Push 3 seconds of zeros
    for (let i = 0; i < SAMPLE_RATE * 3; i++) nm.push(0);
    const result = nm.compute();
    assert.equal(result.tremorClass, 'none');
    assert.equal(result.flagForReview, false);
  });

  it('detects Parkinsonian band signal', () => {
    const nm = new NeuromotorModality();
    // 4.5 Hz — centre of parkinsonian rest-tremor band (3–6 Hz)
    const sig = sineWave(4.5, SAMPLE_RATE * 5);
    sig.forEach(v => nm.push(v));
    const result = nm.compute();
    assert.notEqual(result.tremorDominantHz, 0, 'should detect dominant frequency');
    assert.equal(result.tremorClass, 'parkinsonian');
  });

  it('does not flag healthy profile', () => {
    const nm = new NeuromotorModality();
    // 11 Hz — physiological tremor band only, low amplitude
    const sig = sineWave(11, SAMPLE_RATE * 5).map(v => v * 0.05);
    sig.forEach(v => nm.push(v));
    const result = nm.compute();
    assert.equal(result.flagForReview, false,
      `healthy profile should not be flagged (score: ${result.updrsProxyScore})`);
  });

  it('empty buffer returns zero metrics', () => {
    const nm = new NeuromotorModality();
    const result = nm.compute();
    assert.equal(result.tremorDominantHz, 0);
    assert.equal(result.gaitCadenceSpm, 0);
    assert.equal(result.updrsProxyScore, 0);
  });
});

describe('FFT correctness', () => {
  it('dominant frequency is within 1 Hz of injected frequency', () => {
    const nm = new NeuromotorModality();
    const targetHz = 5.0;
    const sig = sineWave(targetHz, SAMPLE_RATE * 6);
    sig.forEach(v => nm.push(v));
    const result = nm.compute();
    assert(
      Math.abs(result.tremorDominantHz - targetHz) <= 1.0,
      `expected ~${targetHz} Hz, got ${result.tremorDominantHz}`
    );
  });
});
