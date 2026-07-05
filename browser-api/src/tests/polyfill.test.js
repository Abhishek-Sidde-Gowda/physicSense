/**
 * Tests for the PhysicSense polyfill install and session API.
 * Run with: node --test src/tests/polyfill.test.js
 */
import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { PhysicSensePolyfill } from '../polyfill.js';

// Minimal browser API stubs for Node.js test environment
globalThis.navigator = globalThis.navigator ?? {};
globalThis.performance = { now: () => Date.now() };
globalThis.Event = class Event { constructor(type) { this.type = type; } };
globalThis.ErrorEvent = class ErrorEvent extends Event {
  constructor(type, init) { super(type); Object.assign(this, init); }
};
globalThis.EventTarget = class EventTarget {
  constructor() { this._listeners = {}; }
  addEventListener(t, fn) { (this._listeners[t] ??= []).push(fn); }
  removeEventListener(t, fn) {
    this._listeners[t] = (this._listeners[t] ?? []).filter(f => f !== fn);
  }
  dispatchEvent(e) { (this._listeners[e.type] ?? []).forEach(fn => fn(e)); }
};

// Stub AudioContext, getUserMedia (they're not in Node)
globalThis.AudioContext = class {
  constructor() { this.destination = {}; this.sampleRate = 44100; }
  createBuffer(ch, n, sr) {
    return { getChannelData: () => new Float32Array(n) };
  }
  createBufferSource() {
    return { connect() {}, start() {}, set onended(fn) {} };
  }
  createGain() { return { gain: { value: 1 }, connect() {} }; }
  createMediaStreamSource() { return { connect() {} }; }
  createAnalyser() {
    return {
      fftSize: 2048,
      frequencyBinCount: 1024,
      getFloatFrequencyData(arr) { arr.fill(-100); },
      connect() {},
    };
  }
  close() {}
};
globalThis.navigator.mediaDevices = {
  getUserMedia: async () => ({ getTracks: () => [{ stop() {} }] }),
};

describe('PhysicSensePolyfill', () => {
  let api;
  beforeEach(() => { api = new PhysicSensePolyfill(); });

  it('rejects empty modalities', async () => {
    await assert.rejects(
      () => api.requestSession({ modalities: [] }),
      TypeError
    );
  });

  it('rejects missing modalities', async () => {
    await assert.rejects(
      () => api.requestSession({}),
      TypeError
    );
  });

  it('returns a session for acoustic modality', async () => {
    const session = await api.requestSession({ modalities: ['acoustic'] });
    assert.equal(typeof session.start, 'function');
    assert.equal(typeof session.stop,  'function');
    assert.equal(session.active, false);
  });

  it('session becomes active after start', async () => {
    const session = await api.requestSession({ modalities: ['neuromotor'] });
    session.start();
    assert.equal(session.active, true);
    session.stop();
    assert.equal(session.active, false);
  });

  it('queryPermission returns a valid state string', async () => {
    const state = await api.queryPermission({ name: 'physicSense.acoustic' });
    assert(['granted', 'denied', 'prompt'].includes(state),
      `unexpected state: ${state}`);
  });

  it('queryPermission rejects invalid descriptor', async () => {
    await assert.rejects(
      () => api.queryPermission({ name: 'camera' }),
      TypeError
    );
  });
});
