/// Federated aggregator — collects privatised gradient updates from peers
/// and produces a new global model via weighted FedAvg.
///
/// FedAvg (McMahan et al., 2017): the global update is the weighted average
/// of local updates, weighted by each node's sample count.
///
/// Key invariant: the aggregator never sees raw sensing data.
/// It only ever receives GradientUpdates that have already been:
///   1. Computed locally from private data
///   2. Clipped to sensitivity bound C
///   3. Noised by the DP engine
///   4. Integrity-hashed before transmission
use crate::gradient::{GradientUpdate, ModelGradient};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AggregatorError {
    #[error("update from round {got} rejected — aggregator is on round {expected}")]
    WrongRound { expected: u64, got: u64 },
    #[error("gradient integrity check failed for node {0}")]
    IntegrityFailure(String),
    #[error("duplicate update from node {0} in round {1}")]
    DuplicateUpdate(String, u64),
    #[error("need at least {min} updates to aggregate, have {have}")]
    InsufficientUpdates { min: usize, have: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationResult {
    pub round: u64,
    pub participating_nodes: Vec<String>,
    pub total_samples: u64,
    pub global_gradients: Vec<ModelGradient>,
}

pub struct FederatedAggregator {
    round: u64,
    /// Minimum number of nodes before aggregation proceeds.
    min_nodes: usize,
    pending: Vec<GradientUpdate>,
    seen_nodes: std::collections::HashSet<String>,
}

impl FederatedAggregator {
    pub fn new(min_nodes: usize) -> Self {
        Self {
            round: 0,
            min_nodes,
            pending: Vec::new(),
            seen_nodes: std::collections::HashSet::new(),
        }
    }

    pub fn current_round(&self) -> u64 { self.round }
    pub fn pending_count(&self) -> usize { self.pending.len() }

    /// Accept an update from a peer. Validates round, integrity, and dedup.
    pub fn accept(&mut self, update: GradientUpdate) -> Result<(), AggregatorError> {
        if update.round != self.round {
            return Err(AggregatorError::WrongRound {
                expected: self.round,
                got: update.round,
            });
        }

        if !update.verify_integrity() {
            return Err(AggregatorError::IntegrityFailure(update.node_id));
        }

        if self.seen_nodes.contains(&update.node_id) {
            return Err(AggregatorError::DuplicateUpdate(
                update.node_id, self.round,
            ));
        }

        self.seen_nodes.insert(update.node_id.clone());
        self.pending.push(update);
        Ok(())
    }

    /// Run FedAvg aggregation if enough updates have arrived.
    pub fn try_aggregate(&mut self) -> Result<AggregationResult, AggregatorError> {
        if self.pending.len() < self.min_nodes {
            return Err(AggregatorError::InsufficientUpdates {
                min: self.min_nodes,
                have: self.pending.len(),
            });
        }

        let result = self.fedavg();
        self.pending.clear();
        self.seen_nodes.clear();
        self.round += 1;
        Ok(result)
    }

    fn fedavg(&self) -> AggregationResult {
        let total_samples: u64 = self.pending
            .iter()
            .map(|u| u.sample_count as u64)
            .sum();

        let participating_nodes: Vec<String> = self.pending
            .iter()
            .map(|u| u.node_id.clone())
            .collect();

        // Determine layer set from first update
        let n_layers = self.pending[0].gradients.len();
        let mut global: Vec<ModelGradient> = self.pending[0]
            .gradients
            .iter()
            .map(|g| ModelGradient {
                layer: g.layer.clone(),
                values: vec![0.0; g.values.len()],
                pre_clip_norm: 0.0,
            })
            .collect();

        // Weighted sum
        for update in &self.pending {
            let weight = update.sample_count as f32 / total_samples as f32;
            for (layer_idx, grad) in update.gradients.iter().enumerate() {
                if layer_idx >= n_layers { continue; }
                let global_layer = &mut global[layer_idx];
                if global_layer.values.len() != grad.values.len() { continue; }
                for (g, &v) in global_layer.values.iter_mut().zip(grad.values.iter()) {
                    *g += weight * v;
                }
            }
        }

        AggregationResult {
            round: self.round,
            participating_nodes,
            total_samples,
            global_gradients: global,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gradient::GradientUpdate;

    fn make_update(node_id: &str, round: u64, samples: u32, val: f32) -> GradientUpdate {
        let g = ModelGradient::new("layer0", vec![val; 4]).unwrap();
        GradientUpdate::new(node_id, round, samples, vec![g])
    }

    #[test]
    fn accepts_valid_updates() {
        let mut agg = FederatedAggregator::new(2);
        agg.accept(make_update("node-a", 0, 100, 1.0)).unwrap();
        agg.accept(make_update("node-b", 0, 100, 3.0)).unwrap();
        assert_eq!(agg.pending_count(), 2);
    }

    #[test]
    fn rejects_wrong_round() {
        let mut agg = FederatedAggregator::new(1);
        let err = agg.accept(make_update("node-a", 5, 100, 1.0));
        assert!(matches!(err, Err(AggregatorError::WrongRound { .. })));
    }

    #[test]
    fn rejects_duplicate_node() {
        let mut agg = FederatedAggregator::new(2);
        agg.accept(make_update("node-a", 0, 100, 1.0)).unwrap();
        let err = agg.accept(make_update("node-a", 0, 100, 2.0));
        assert!(matches!(err, Err(AggregatorError::DuplicateUpdate(_, _))));
    }

    #[test]
    fn fedavg_weighted_correctly() {
        let mut agg = FederatedAggregator::new(2);
        // node-a: 100 samples, gradient = 1.0
        // node-b: 300 samples, gradient = 3.0
        // weighted avg = (100*1 + 300*3) / 400 = 1000/400 = 2.5
        agg.accept(make_update("node-a", 0, 100, 1.0)).unwrap();
        agg.accept(make_update("node-b", 0, 300, 3.0)).unwrap();
        let result = agg.try_aggregate().unwrap();
        let avg = result.global_gradients[0].values[0];
        assert!((avg - 2.5).abs() < 1e-4, "weighted avg should be 2.5, got {avg}");
    }

    #[test]
    fn round_increments_after_aggregation() {
        let mut agg = FederatedAggregator::new(1);
        agg.accept(make_update("node-a", 0, 50, 1.0)).unwrap();
        agg.try_aggregate().unwrap();
        assert_eq!(agg.current_round(), 1);
    }

    #[test]
    fn insufficient_updates_returns_error() {
        let mut agg = FederatedAggregator::new(3);
        agg.accept(make_update("node-a", 0, 50, 1.0)).unwrap();
        assert!(matches!(
            agg.try_aggregate(),
            Err(AggregatorError::InsufficientUpdates { .. })
        ));
    }
}
