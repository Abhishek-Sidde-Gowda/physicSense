'use strict';

/**
 * WebRTC data channel transport for the federated mesh.
 *
 * Each browser node:
 *  1. Connects to a lightweight signalling server (WebSocket) for SDP exchange.
 *  2. Establishes a direct peer-to-peer RTCDataChannel with each other node.
 *  3. Sends FederatedMessage JSON over the data channel — no server relay.
 *
 * The signalling server only sees: peer IDs and SDP blobs.
 * Gradient updates travel peer-to-peer — the server never sees them.
 */

const ICE_SERVERS = [
  { urls: 'stun:stun.l.google.com:19302' },
  { urls: 'stun:stun1.l.google.com:19302' },
];

export class WebRtcTransport {
  /**
   * @param {string} signalingUrl  WebSocket URL of the signalling server.
   * @param {string} localPeerId   This node's peer ID string.
   */
  constructor(signalingUrl, localPeerId) {
    this._signalingUrl  = signalingUrl;
    this._localPeerId   = localPeerId;
    this._ws            = null;
    this._connections   = new Map(); // peerId → { pc, channel }
    this._onMessage     = null;      // callback(peerId, FederatedMessage)
    this._onPeerJoin    = null;      // callback(peerId)
    this._onPeerLeave   = null;      // callback(peerId)
  }

  /** Connect to signalling server and announce presence. */
  async connect() {
    this._ws = new WebSocket(this._signalingUrl);

    await new Promise((resolve, reject) => {
      this._ws.onopen  = resolve;
      this._ws.onerror = reject;
    });

    this._ws.onmessage = (event) => this._handleSignal(JSON.parse(event.data));
    this._ws.onclose   = () => this._cleanup();

    this._signal({ type: 'hello', from: this._localPeerId });
  }

  /** Send a FederatedMessage to a specific peer. */
  send(peerId, message) {
    const conn = this._connections.get(peerId);
    if (!conn?.channel || conn.channel.readyState !== 'open') {
      console.warn(`[federated] channel to ${peerId} not open`);
      return false;
    }
    conn.channel.send(JSON.stringify(message));
    return true;
  }

  /** Broadcast a FederatedMessage to all connected peers. */
  broadcast(message) {
    for (const [peerId] of this._connections) {
      this.send(peerId, message);
    }
  }

  /** Set handler called when a message arrives from any peer. */
  onMessage(fn) { this._onMessage = fn; }
  onPeerJoin(fn) { this._onPeerJoin = fn; }
  onPeerLeave(fn) { this._onPeerLeave = fn; }

  connectedPeers() {
    return [...this._connections.keys()].filter(id => {
      const c = this._connections.get(id);
      return c?.channel?.readyState === 'open';
    });
  }

  disconnect() {
    this._cleanup();
    this._ws?.close();
  }

  // --- private ---

  async _handleSignal(signal) {
    switch (signal.type) {
      case 'peer_joined':
        await this._initiate(signal.peerId);
        break;

      case 'offer':
        await this._handleOffer(signal.from, signal.sdp);
        break;

      case 'answer':
        await this._connections.get(signal.from)?.pc
          .setRemoteDescription({ type: 'answer', sdp: signal.sdp });
        break;

      case 'ice':
        await this._connections.get(signal.from)?.pc
          .addIceCandidate(signal.candidate);
        break;

      case 'peer_left':
        this._closePeer(signal.peerId);
        break;
    }
  }

  async _initiate(remotePeerId) {
    const pc = this._createPc(remotePeerId);
    const channel = pc.createDataChannel('federated', { ordered: true });
    this._connections.set(remotePeerId, { pc, channel });
    this._wireChannel(remotePeerId, channel);

    const offer = await pc.createOffer();
    await pc.setLocalDescription(offer);
    this._signal({ type: 'offer', to: remotePeerId, from: this._localPeerId, sdp: offer.sdp });
  }

  async _handleOffer(remotePeerId, sdp) {
    const pc = this._createPc(remotePeerId);
    this._connections.set(remotePeerId, { pc, channel: null });

    pc.ondatachannel = ({ channel }) => {
      this._connections.get(remotePeerId).channel = channel;
      this._wireChannel(remotePeerId, channel);
    };

    await pc.setRemoteDescription({ type: 'offer', sdp });
    const answer = await pc.createAnswer();
    await pc.setLocalDescription(answer);
    this._signal({ type: 'answer', to: remotePeerId, from: this._localPeerId, sdp: answer.sdp });
  }

  _createPc(remotePeerId) {
    const pc = new RTCPeerConnection({ iceServers: ICE_SERVERS });
    pc.onicecandidate = ({ candidate }) => {
      if (candidate) {
        this._signal({ type: 'ice', to: remotePeerId, from: this._localPeerId, candidate });
      }
    };
    return pc;
  }

  _wireChannel(remotePeerId, channel) {
    channel.onopen = () => {
      this._onPeerJoin?.(remotePeerId);
    };
    channel.onclose = () => {
      this._closePeer(remotePeerId);
    };
    channel.onmessage = ({ data }) => {
      try {
        const msg = JSON.parse(data);
        this._onMessage?.(remotePeerId, msg);
      } catch {
        console.warn('[federated] malformed message from', remotePeerId);
      }
    };
  }

  _closePeer(peerId) {
    const conn = this._connections.get(peerId);
    if (conn) {
      conn.channel?.close();
      conn.pc?.close();
      this._connections.delete(peerId);
      this._onPeerLeave?.(peerId);
    }
  }

  _cleanup() {
    for (const peerId of this._connections.keys()) {
      this._closePeer(peerId);
    }
  }

  _signal(msg) {
    if (this._ws?.readyState === WebSocket.OPEN) {
      this._ws.send(JSON.stringify(msg));
    }
  }
}
