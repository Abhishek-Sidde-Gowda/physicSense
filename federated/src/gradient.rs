/// Local model gradient — the only thing that ever leaves a node.
///
/// Raw sensing data (IQ samples, acoustic captures, phase signals) never
/// leaves the device. Only gradient updates computed from local data are
/// shared, and those are further protected by differential privacy noise
/// before transmission.
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GradientError {
    #[error("gradient vectors must have equal length, got {0} vs {1}")]
    LengthMismatch(usize, usize),
    #[error("gradient contains NaN or Inf at index {0}")]
    InvalidValue(usize),
}

/// A flat vector of gradient values for one model layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelGradient {
    /// Layer identifier (e.g. "tremor_bandpass.weight").
    pub layer: String,
    /// Gradient values — f32 for bandwidth efficiency.
    pub values: Vec<f32>,
    /// L2 norm of values before clipping (used for privacy accounting).
    pub pre_clip_norm: f32,
}

impl ModelGradient {
    pub fn new(layer: impl Into<String>, values: Vec<f32>) -> Result<Self, GradientError> {
        for (i, &v) in values.iter().enumerate() {
            if !v.is_finite() {
                return Err(GradientError::InvalidValue(i));
            }
        }
        let pre_clip_norm = l2_norm(&values);
        Ok(Self { layer: layer.into(), values, pre_clip_norm })
    }

    pub fn len(&self) -> usize { self.values.len() }
    pub fn is_empty(&self) -> bool { self.values.is_empty() }

    /// Clip gradient to L2 norm `c` (standard DP-SGD clipping step).
    pub fn clip(&mut self, c: f32) {
        let norm = l2_norm(&self.values);
        if norm > c {
            let scale = c / norm;
            self.values.iter_mut().for_each(|v| *v *= scale);
        }
    }

    /// Element-wise add another gradient (for local accumulation).
    pub fn add(&mut self, other: &ModelGradient) -> Result<(), GradientError> {
        if self.values.len() != other.values.len() {
            return Err(GradientError::LengthMismatch(self.values.len(), other.values.len()));
        }
        self.values.iter_mut().zip(other.values.iter()).for_each(|(a, b)| *a += b);
        Ok(())
    }

    /// Scale all values by a scalar (used for weighted averaging).
    pub fn scale(&mut self, factor: f32) {
        self.values.iter_mut().for_each(|v| *v *= factor);
    }
}

/// A complete gradient update from one node for one training round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientUpdate {
    pub node_id: String,
    pub round: u64,
    /// Number of local samples used to compute this gradient.
    pub sample_count: u32,
    pub gradients: Vec<ModelGradient>,
    /// Blake3 hash of the serialised gradients (integrity check).
    pub integrity_hash: String,
}

impl GradientUpdate {
    pub fn new(
        node_id: impl Into<String>,
        round: u64,
        sample_count: u32,
        gradients: Vec<ModelGradient>,
    ) -> Self {
        let node_id = node_id.into();
        let hash = Self::compute_hash(&gradients);
        Self { node_id, round, sample_count, gradients, integrity_hash: hash }
    }

    pub fn verify_integrity(&self) -> bool {
        Self::compute_hash(&self.gradients) == self.integrity_hash
    }

    fn compute_hash(gradients: &[ModelGradient]) -> String {
        // Lightweight hash using sum of values — production would use BLAKE3.
        // BLAKE3 requires a native crate; this avoids the dependency for now
        // while preserving the interface for a future swap.
        let mut acc: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
        for g in gradients {
            for &v in &g.values {
                let bits = v.to_bits() as u64;
                acc ^= bits;
                acc = acc.wrapping_mul(0x100000001b3);
            }
        }
        format!("{acc:016x}")
    }
}

pub fn l2_norm(v: &[f32]) -> f32 {
    v.iter().map(|&x| x * x).sum::<f32>().sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_reduces_large_gradient() {
        let mut g = ModelGradient::new("test", vec![3.0, 4.0]).unwrap();
        g.clip(1.0);
        let norm = l2_norm(&g.values);
        assert!((norm - 1.0).abs() < 1e-5, "clipped norm should be 1.0, got {norm}");
    }

    #[test]
    fn clip_leaves_small_gradient_unchanged() {
        let mut g = ModelGradient::new("test", vec![0.1, 0.1]).unwrap();
        let before = g.values.clone();
        g.clip(10.0);
        assert_eq!(g.values, before);
    }

    #[test]
    fn rejects_nan() {
        assert!(ModelGradient::new("test", vec![f32::NAN]).is_err());
    }

    #[test]
    fn integrity_hash_detects_tampering() {
        let g = ModelGradient::new("layer", vec![1.0, 2.0, 3.0]).unwrap();
        let mut update = GradientUpdate::new("node-1", 0, 10, vec![g]);
        assert!(update.verify_integrity());
        update.gradients[0].values[0] = 99.0;
        assert!(!update.verify_integrity(), "tampered gradient should fail hash check");
    }
}
