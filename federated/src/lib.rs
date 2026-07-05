pub mod gradient;
pub mod privacy;
pub mod aggregator;
pub mod peer;
pub mod protocol;

pub use gradient::{ModelGradient, GradientUpdate};
pub use privacy::DifferentialPrivacy;
pub use aggregator::{FederatedAggregator, AggregationResult};
pub use peer::{PeerId, PeerRegistry};
pub use protocol::{FederatedMessage, MessageType};
