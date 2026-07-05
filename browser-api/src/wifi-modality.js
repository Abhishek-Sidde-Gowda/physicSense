'use strict';

const EXTENSION_ID_KEY = 'physicSense.extensionId';

/**
 * WiFi passive PCL modality — bridges raw 802.11 IQ frames from the
 * WebExtension native messaging host into the page context.
 *
 * The extension sends messages of the form:
 *   { type: "wifi_frame", timestamp_us, channel, rssi_dbm, payload_b64 }
 *
 * The polyfill listens via window.addEventListener("message") from the
 * extension content script (which relays from background via postMessage).
 */
export class WifiModality {
  constructor() {
    this._frames = [];
    this._maxFrames = 512;
    this._listener = null;
    this._available = false;
  }

  /**
   * Check whether the PhysicSense extension is installed and responding.
   * @returns {Promise<boolean>}
   */
  async checkExtension() {
    return new Promise(resolve => {
      const timeout = setTimeout(() => resolve(false), 1000);

      const handler = (event) => {
        if (event.data?.type === 'physicSense_pong') {
          clearTimeout(timeout);
          window.removeEventListener('message', handler);
          this._available = true;
          resolve(true);
        }
      };
      window.addEventListener('message', handler);
      window.postMessage({ type: 'physicSense_ping' }, '*');
    });
  }

  start() {
    this._listener = (event) => {
      if (event.data?.type !== 'physicSense_wifi_frame') return;
      const frame = this._decodeFrame(event.data);
      if (!frame) return;
      this._frames.push(frame);
      if (this._frames.length > this._maxFrames) this._frames.shift();
    };
    window.addEventListener('message', this._listener);
  }

  stop() {
    if (this._listener) {
      window.removeEventListener('message', this._listener);
      this._listener = null;
    }
    this._frames = [];
  }

  /**
   * Drain accumulated frames for one CPI.
   * @returns {Float32Array[]} array of IQ sample arrays
   */
  drainCpi() {
    const batch = this._frames.splice(0);
    return batch.map(f => f.iq);
  }

  get available() { return this._available; }

  _decodeFrame(data) {
    try {
      const raw = atob(data.payload_b64);
      const iq = new Float32Array(raw.length / 4);
      const view = new DataView(
        Uint8Array.from(raw, c => c.charCodeAt(0)).buffer
      );
      for (let i = 0; i < iq.length; i++) {
        iq[i] = view.getFloat32(i * 4, true);
      }
      return { timestamp_us: data.timestamp_us, channel: data.channel, iq };
    } catch {
      return null;
    }
  }
}
