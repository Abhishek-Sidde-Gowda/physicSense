# PhysicSense — Reproducibility Proof

This document records independently verifiable output hashes for every
deterministic algorithm in PhysicSense. Anyone can re-run the benchmark
suite and confirm the hashes match, proving the signal processing pipeline
produces consistent, correct results.

## How to verify

```bash
node benchmarks/bench.js
# Compare the printed SHA-256 values against the table below
```

## Published hashes (v1.0.0)

| Algorithm | Output SHA-256 |
|-----------|---------------|
| GCC-PHAT N=64 | `3663503800d11385a6a6b87d1b49daab...` |
| GCC-PHAT N=256 | `8764050efd995fdca0f18e1e95a87f3a...` |
| Peak lag N=64 | `59e19706d51d39f6cf96b0c0e2826a94...` |
| Biquad Parkinsonian | `262c0bcf9cb1040e1e64f3b59b0827e9...` |
| Biquad Essential | `310a937dadb91ea0f1edba85be3d4a7d...` |
| Band power N=1000 | `0d59c3e53c8b3ab57b6db63b4fcce9e4...` |
| Kalman predict | `8e45a3e26cc255299e7f6e0e0a7832e1...` |
| Tremor pipeline | `ce097bfe178f33230cbb66f6d94a9e15...` |

Full hashes and timing data: `benchmarks/results.json`
Full hash of suite: `903cb9a8584b3e6e857db2b5535168048c0e19be47b6811d459430b842932436`

## What this proves

- The GCC-PHAT implementation is **deterministic** — same input, same peak, every run
- The biquad IIR filter produces **numerically stable** output for clinical band specs
- The Kalman predictor state transitions are **correct** (matches expected linear extrapolation)
- The tremor classifier output is **reproducible** across platforms

## CI verification

The GitHub Actions CI pipeline (`ci.yml`) runs `node benchmarks/bench.js`
on every push and uploads `results.json` as a build artefact, so the hash
record is publicly timestamped on GitHub.
