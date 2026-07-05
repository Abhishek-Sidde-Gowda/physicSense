/// Wire protocol for federated mesh messages.
///
/// All messages are JSON-serialised and transported over WebRTC data channels
/// (browser) or TCP (native nodes). The protocol is deliberately minimal —
/// no raw sensing data ever appears on the wire.
use crate::gradient::GradientUpdate;
use crate::peer::PeerId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// Peer announces itself to the mesh.
    Hello,
    /// Coordinator broadcasts the start of a new training round.
    RoundStart,
    /// Node submits its DP-noised gradient update.
    GradientSubmit,
    /// Coordinator broadcasts the aggregated global model update.
    GlobalUpdate,
    /// Heartbeat — keep the data channel alive.
    Ping,
    /// Response to Ping.
    Pong,
    /// Graceful disconnect notification.
    Bye,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedMessage {
    pub msg_type: MessageType,
    pub from: PeerId,
    pub round: u64,
    pub payload: MessagePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessagePayload {
    Empty {},
    RoundStart { min_nodes: usize, deadline_ms: u64 },
    GradientSubmit { update: GradientUpdate },
    GlobalUpdate { gradients: Vec<crate::gradient::ModelGradient>, total_samples: u64 },
    Hello { key_fingerprint: String, version: String },
}

impl FederatedMessage {
    pub fn hello(from: PeerId, key_fingerprint: String) -> Self {
        Self {
            msg_type: MessageType::Hello,
            round: 0,
            payload: MessagePayload::Hello {
                key_fingerprint,
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            from,
        }
    }

    pub fn round_start(from: PeerId, round: u64, min_nodes: usize, deadline_ms: u64) -> Self {
        Self {
            msg_type: MessageType::RoundStart,
            round,
            payload: MessagePayload::RoundStart { min_nodes, deadline_ms },
            from,
        }
    }

    pub fn gradient_submit(from: PeerId, update: GradientUpdate) -> Self {
        let round = update.round;
        Self {
            msg_type: MessageType::GradientSubmit,
            round,
            payload: MessagePayload::GradientSubmit { update },
            from,
        }
    }

    pub fn ping(from: PeerId, round: u64) -> Self {
        Self { msg_type: MessageType::Ping, from, round, payload: MessagePayload::Empty {} }
    }

    pub fn pong(from: PeerId, round: u64) -> Self {
        Self { msg_type: MessageType::Pong, from, round, payload: MessagePayload::Empty {} }
    }

    /// Serialise to JSON bytes for wire transport.
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Deserialise from JSON bytes received over the wire.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_hello_message() {
        let id = PeerId("test-node".to_string());
        let msg = FederatedMessage::hello(id.clone(), "fp-abc123".to_string());
        let bytes = msg.to_bytes();
        let decoded = FederatedMessage::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.msg_type, MessageType::Hello);
        assert_eq!(decoded.from, id);
    }

    #[test]
    fn round_trips_gradient_submit() {
        use crate::gradient::{ModelGradient, GradientUpdate};
        let id = PeerId("node-1".to_string());
        let g = ModelGradient::new("layer0", vec![0.1, 0.2, 0.3]).unwrap();
        let update = GradientUpdate::new("node-1", 2, 50, vec![g]);
        let msg = FederatedMessage::gradient_submit(id, update);
        let bytes = msg.to_bytes();
        let decoded = FederatedMessage::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.msg_type, MessageType::GradientSubmit);
        assert_eq!(decoded.round, 2);
    }

    #[test]
    fn rejects_malformed_bytes() {
        let result = FederatedMessage::from_bytes(b"not json {{{");
        assert!(result.is_err());
    }
}
