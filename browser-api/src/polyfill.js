'use strict';

import { AcousticModality }    from './acoustic-modality.js';
import { WifiModality }        from './wifi-modality.js';
import { NeuromotorModality }  from './neuromotor-modality.js';

const FRAME_INTERVAL_MS = 100; // ~10 Hz output rate

class PhysicSenseFrameEvent extends Event {
  constructor(frame) {
    super('frame');
    this.frame = frame;
  }
}

class PhysicSenseSession extends EventTarget {
  constructor(init, acoustic, wifi, neuromotor) {
    super();
    this._init        = init;
    this._acoustic    = acoustic;
    this._wifi        = wifi;
    this._neuromotor  = neuromotor;
    this._active      = false;
    this._timer       = null;
    this.onframe      = null;
    this.onerror      = null;
  }

  get active() { return this._active; }

  start() {
    if (this._active) return;
    this._active = true;

    if (this._init.modalities.includes('acoustic')) this._acoustic.start();
    if (this._init.modalities.includes('wifi'))     this._wifi.start();

    this._timer = setInterval(() => this._tick(), FRAME_INTERVAL_MS);
  }

  stop() {
    if (!this._active) return;
    this._active = false;
    clearInterval(this._timer);
    if (this._init.modalities.includes('acoustic')) this._acoustic.stop();
    if (this._init.modalities.includes('wifi'))     this._wifi.stop();
  }

  _tick() {
    try {
      const timestamp = performance.now();

      // --- Acoustic ---
      let rangeM = null, position2d = null, beatSpectrum = null;
      if (this._init.modalities.includes('acoustic')) {
        const r = this._acoustic.readFrame();
        rangeM = r.rangeM;
        // Feed range as phase displacement to neuromotor
        if (rangeM !== null) this._neuromotor.push(rangeM);
      }

      // --- WiFi ---
      let rangeDopplerMap = null, velocityMs = null;
      if (this._init.modalities.includes('wifi')) {
        const cpiFrames = this._wifi.drainCpi();
        if (cpiFrames.length > 0) {
          // Placeholder: real processing is done in WASM fusion engine.
          // Here we return a zero-filled map until WASM is loaded.
          rangeDopplerMap = new Float32Array(64 * 8);
          velocityMs      = 0;
        }
      }

      // --- Neuromotor ---
      let neuromotor = null;
      if (this._init.modalities.includes('neuromotor')) {
        neuromotor = this._neuromotor.compute();
      }

      const frame = {
        timestamp,
        rangeDopplerMap,
        velocityMs,
        rangeM,
        position2d,
        neuromotor,
      };

      const event = new PhysicSenseFrameEvent(frame);
      this.dispatchEvent(event);
      if (this.onframe) this.onframe(event);

    } catch (err) {
      const errorEvent = new ErrorEvent('error', { error: err, message: err.message });
      this.dispatchEvent(errorEvent);
      if (this.onerror) this.onerror(errorEvent);
    }
  }
}

class PhysicSensePolyfill extends EventTarget {
  async requestSession(init) {
    if (!init?.modalities?.length) {
      throw new TypeError('modalities must be a non-empty array');
    }

    const acoustic   = new AcousticModality(init.sampleRate ?? 44100);
    const wifi       = new WifiModality();
    const neuromotor = new NeuromotorModality();

    if (init.modalities.includes('acoustic')) {
      await acoustic.init();
    }

    if (init.modalities.includes('wifi')) {
      const available = await wifi.checkExtension();
      if (!available) {
        console.warn(
          '[PhysicSense] WiFi extension not found. ' +
          'Install the PhysicSense browser extension for passive WiFi sensing. ' +
          'Continuing with acoustic + neuromotor only.'
        );
      }
    }

    return new PhysicSenseSession(init, acoustic, wifi, neuromotor);
  }

  async queryPermission(descriptor) {
    if (!descriptor?.name?.startsWith('physicSense')) {
      throw new TypeError('Invalid PhysicSense permission descriptor');
    }
    // Delegate to the Permissions API where available
    if ('permissions' in navigator) {
      try {
        const result = await navigator.permissions.query({ name: descriptor.name });
        return result.state;
      } catch {
        // Browser doesn't know this permission yet — return 'prompt'
      }
    }
    return 'prompt';
  }
}

/**
 * Install navigator.physicSense if not already defined.
 * Call this once at page load: import and call installPolyfill().
 */
export function installPolyfill() {
  if ('physicSense' in navigator) return;

  Object.defineProperty(navigator, 'physicSense', {
    value: new PhysicSensePolyfill(),
    writable: false,
    configurable: false,
    enumerable: true,
  });
}

export { PhysicSensePolyfill, PhysicSenseSession };
