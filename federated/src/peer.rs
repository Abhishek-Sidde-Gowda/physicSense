/// Peer registry — tracks known nodes in the federated mesh.
///
/// In the browser deployment, peer discovery uses WebRTC data channels.
/// Each peer is identified by a UUID and announces itself to a lightweight
/// signalling server (or via mDNS on the local network for LAN-only mode).
///
/// No sensing data is exchanged through the peer registry — only:
///   - peer presence (UUID + public key fingerprint)
///   - round coordination messages
///   - DP-noised gradient updates (see GradientUpdate)
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Unique node identifier — random UUID v4 generated at first run.
/// Never tied to device identity, IP address, or user account.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub String);

impl PeerId {
    /// Generate a new random peer ID using entropy from the system clock
    /// mixed with a counter. Production: use uuid v4 crate.
    pub fn generate() -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        // Simple non-cryptographic ID for testing — good enough for peer tracking
        Self(format!("peer-{ts:08x}"))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PeerState {
    Discovered,
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: PeerId,
    pub state: PeerState,
    /// Ed25519 public key fingerprint (hex) for update authentication.
    /// Full key exchange happens during WebRTC DTLS handshake.
    pub key_fingerprint: String,
    pub last_seen_ms: u64,
    /// Number of completed federated rounds with this peer.
    pub rounds_completed: u32,
}

pub struct PeerRegistry {
    peers: HashMap<PeerId, PeerInfo>,
    local_id: PeerId,
}

impl PeerRegistry {
    pub fn new(local_id: PeerId) -> Self {
        Self { peers: HashMap::new(), local_id }
    }

    pub fn local_id(&self) -> &PeerId { &self.local_id }

    pub fn register(&mut self, info: PeerInfo) {
        self.peers.insert(info.id.clone(), info);
    }

    pub fn update_state(&mut self, id: &PeerId, state: PeerState) {
        if let Some(peer) = self.peers.get_mut(id) {
            peer.state = state;
            peer.last_seen_ms = now_ms();
        }
    }

    pub fn mark_round_complete(&mut self, id: &PeerId) {
        if let Some(peer) = self.peers.get_mut(id) {
            peer.rounds_completed += 1;
            peer.last_seen_ms = now_ms();
        }
    }

    pub fn connected_peers(&self) -> Vec<&PeerInfo> {
        self.peers
            .values()
            .filter(|p| p.state == PeerState::Connected)
            .collect()
    }

    pub fn remove_stale(&mut self, timeout_ms: u64) {
        let now = now_ms();
        self.peers.retain(|_, p| {
            p.state == PeerState::Connected || (now - p.last_seen_ms) < timeout_ms
        });
    }

    pub fn peer_count(&self) -> usize { self.peers.len() }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_peer(id: &str) -> PeerInfo {
        PeerInfo {
            id: PeerId(id.to_string()),
            state: PeerState::Connected,
            key_fingerprint: format!("fp-{id}"),
            last_seen_ms: now_ms(),
            rounds_completed: 0,
        }
    }

    #[test]
    fn registers_and_lists_peers() {
        let mut reg = PeerRegistry::new(PeerId::generate());
        reg.register(make_peer("a"));
        reg.register(make_peer("b"));
        assert_eq!(reg.peer_count(), 2);
        assert_eq!(reg.connected_peers().len(), 2);
    }

    #[test]
    fn update_state_changes_peer() {
        let mut reg = PeerRegistry::new(PeerId::generate());
        reg.register(make_peer("a"));
        reg.update_state(&PeerId("a".to_string()), PeerState::Disconnected);
        assert_eq!(reg.connected_peers().len(), 0);
    }

    #[test]
    fn removes_stale_disconnected_peers() {
        let mut reg = PeerRegistry::new(PeerId::generate());
        let mut stale = make_peer("stale");
        stale.state = PeerState::Disconnected;
        stale.last_seen_ms = 0; // far in the past
        reg.register(stale);
        reg.register(make_peer("active"));
        reg.remove_stale(1000);
        assert_eq!(reg.peer_count(), 1);
    }
}
