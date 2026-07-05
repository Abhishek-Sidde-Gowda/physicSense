/// Scaled dot-product attention for multi-modal feature fusion.
///
/// Each sensing modality (WiFi PCL, acoustic, neuromotor) contributes a
/// feature vector. Attention weights are learned to suppress noisy modalities
/// (e.g. acoustic when the room is reverberant) and amplify confident ones.
use std::f32;

pub fn softmax(v: &[f32]) -> Vec<f32> {
    let max = v.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = v.iter().map(|&x| (x - max).exp()).collect();
    let sum: f32 = exps.iter().sum();
    exps.iter().map(|&e| e / sum).collect()
}

/// Scaled dot-product attention score between query `q` and key `k`.
pub fn attention_score(q: &[f32], k: &[f32]) -> f32 {
    assert_eq!(q.len(), k.len());
    let d = q.len() as f32;
    let dot: f32 = q.iter().zip(k.iter()).map(|(&a, &b)| a * b).sum();
    dot / d.sqrt()
}

/// Fuse N modality vectors using learned query weights.
/// Returns a weighted sum of value vectors.
pub fn fuse(
    query: &[f32],
    keys: &[&[f32]],
    values: &[&[f32]],
) -> Vec<f32> {
    assert_eq!(keys.len(), values.len());
    assert!(!keys.is_empty());

    let dim = values[0].len();
    let scores: Vec<f32> = keys.iter().map(|k| attention_score(query, k)).collect();
    let weights = softmax(&scores);

    let mut out = vec![0.0f32; dim];
    for (w, v) in weights.iter().zip(values.iter()) {
        for (o, &val) in out.iter_mut().zip(v.iter()) {
            *o += w * val;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn softmax_sums_to_one() {
        let v = vec![1.0, 2.0, 3.0];
        let s = softmax(&v);
        let sum: f32 = s.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
    }

    #[test]
    fn identical_keys_give_uniform_weights() {
        let q = vec![1.0, 0.0];
        let k = vec![1.0, 0.0];
        let keys: Vec<&[f32]> = vec![&k, &k, &k];
        let v0 = vec![1.0, 0.0];
        let v1 = vec![0.0, 1.0];
        let v2 = vec![0.5, 0.5];
        let values: Vec<&[f32]> = vec![&v0, &v1, &v2];
        let out = fuse(&q, &keys, &values);
        // uniform weights → mean of values
        assert!((out[0] - 0.5).abs() < 0.01);
        assert!((out[1] - 0.5).abs() < 0.01);
    }
}
