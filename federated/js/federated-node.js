'use strict';

import { WebRtcTransport } from './webrtc-transport.js';

/**
 * FederatedNode — the browser-side federated learning participant.
 *
 * Lifecycle per round:
 *  1. Coordinator broadcasts RoundStart
 *  2. Each node trains locally on its buffered sensing data
 *  3. Each node clips + DP-noises its gradient (in WASM)
 *  4. Each node submits GradientUpdate to coordinator via WebRTC
 *  5. Coordinator aggregates and broadcasts GlobalUpdate
 *  6. Each node applies the global update to its local model
 *
 * No raw sensing data leaves this node at any step.
 */
export class FederatedNode {
  /**
   * @param {string} signalingUrl
   * @param {string} peerId
   * @param {{ epsilon: number, delta: number, clipNorm: number }} privacyBudget
   */
  constructor(signalingUrl, peerId, privacyBudget = { epsilon: 1.0, delta: 1e-5, clipNorm: 1.0 }) {
    this._peerId        = peerId;
    this._privacyBudget = privacyBudget;
    this._transport     = new WebRtcTransport(signalingUrl, peerId);
    this._round         = 0;
    this._localSamples  = 0;
    this._onGlobalUpdate = null;
    this._wasmFusion    = null; // set via setWasmModule()
  }

  /** Inject the loaded WASM fusion module (provides gradient computation + DP). */
  setWasmModule(wasmModule) {
    this._wasmFusion = wasmModule;
  }

  async connect() {
    this._transport.onMessage((peerId, msg) => this._handleMessage(peerId, msg));
    this._transport.onPeerJoin((id)  => console.log(`[federated] peer joined: ${id}`));
    this._transport.onPeerLeave((id) => console.log(`[federated] peer left: ${id}`));
    await this._transport.connect();
  }

  /** Record that N new local samples were processed this round. */
  recordSamples(n) { this._localSamples += n; }

  /** Called when global model weights arrive after aggregation. */
  onGlobalUpdate(fn) { this._onGlobalUpdate = fn; }

  disconnect() { this._transport.disconnect(); }

  connectedPeers() { return this._transport.connectedPeers(); }

  // --- private ---

  _handleMessage(fromPeerId, msg) {
    switch (msg.msg_type) {
      case 'round_start':
        this._onRoundStart(msg);
        break;

      case 'global_update':
        this._onReceiveGlobalUpdate(msg);
        break;

      case 'ping':
        this._transport.send(fromPeerId, {
          msg_type: 'pong',
          from: this._peerId,
          round: this._round,
          payload: {},
        });
        break;
    }
  }

  async _onRoundStart(msg) {
    this._round = msg.round;
    console.log(`[federated] round ${this._round} started`);

    // Compute gradient from locally buffered sensing data
    const gradient = await this._computeLocalGradient();
    if (!gradient) return;

    // Submit to coordinator (coordinator peer is the one who sent RoundStart)
    const update = {
      node_id: this._peerId,
      round: this._round,
      sample_count: this._localSamples,
      gradients: [gradient],
      integrity_hash: this._hash(gradient.values),
    };

    this._transport.broadcast({
      msg_type: 'gradient_submit',
      from: this._peerId,
      round: this._round,
      payload: { update },
    });

    this._localSamples = 0;
  }

  _onReceiveGlobalUpdate(msg) {
    console.log(`[federated] round ${msg.round} global update received`);
    this._onGlobalUpdate?.(msg.payload);
  }

  async _computeLocalGradient() {
    if (this._wasmFusion) {
      // Delegate to the Rust WASM module for proper DP-SGD
      return this._wasmFusion.compute_gradient(this._privacyBudget);
    }

    // JS fallback: return a zero gradient (no-op update) when WASM not loaded
    console.warn('[federated] WASM not loaded — submitting zero gradient');
    return {
      layer: 'neuromotor.tremor_head.weight',
      values: new Array(64).fill(0),
      pre_clip_norm: 0,
    };
  }

  /** FNV-1a hash for integrity tagging — matches Rust implementation. */
  _hash(values) {
    let acc = BigInt('0xcbf29ce484222325');
    const mask = BigInt('0xffffffffffffffff');
    const mul  = BigInt('0x100000001b3');
    const buf  = new Float32Array(values);
    const view = new DataView(buf.buffer);
    for (let i = 0; i < buf.length; i++) {
      const bits = BigInt(view.getUint32(i * 4, true));
      acc = ((acc ^ bits) * mul) & mask;
    }
    return acc.toString(16).padStart(16, '0');
  }
}
