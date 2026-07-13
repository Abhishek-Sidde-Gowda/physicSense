# PhysicSense — Benchmark Proof

Generated: 2026-07-13T19:21:42.487Z
Platform: darwin / Node v24.18.0
Suite SHA-256: `903cb9a8584b3e6e857db2b5535168048c0e19be47b6811d459430b842932436`

## Results

| Benchmark | Latency | Throughput | Output SHA-256 |
|-----------|---------|------------|----------------|
| gcc_phat_n64 | 133.4 µs | 7493.5 /s | `3663503800d11385c92de98fa0804e15c746bf830cd41ae4bb2438d75fa8a8bb` |
| gcc_phat_n256 | 2134.8 µs | 468.4 /s | `8764050efd995fdc53ed66c31e2ee52890a3412fe4e5222c9ebe3b9167156c26` |
| peak_lag_n64 | 0.1 µs | 10641128 /s | `59e19706d51d39f66711c2653cd7eb1291c94d9b55eb14bda74ce4dc636d015a` |
| biquad_parkinsonian | 5 µs | 200026.7 /s | `262c0bcf9cb1040e2684d9049fcfce7bd3b3054cb765f9811b170ebeae18968e` |
| biquad_essential | 4.8 µs | 209289 /s | `310a937dadb91ea0ded79d2cf8b5059b6d770fc5cad86ab590548b49726d90a6` |
| band_power_n1000 | 6 µs | 165820.2 /s | `0d59c3e53c8b3ab560fe36e31690f9fe26bb8da661197ebdf6e467e4f1e1bb0b` |
| kalman_predict | 0 µs | 44423042.4 /s | `8e45a3e26cc25529e1e7b057d262586108eb674b4e0fc4e80bdf8bbae069f3cf` |
| tremor_pipeline | 30.8 µs | 32500 /s | `ce097bfe178f33233f9ae6bb09135e8d8294edf2a5371e376b90cc2120065e44` |

## Reproducibility

Run `node benchmarks/bench.js` on any machine to regenerate.
The SHA-256 hashes above are deterministic — identical signal inputs
(seeded PRNG 0xdeadbeef) must produce identical hashes on every platform.

## What is verified

- **GCC-PHAT** cross-correlation output is stable and peak is detectable
- **Biquad IIR** filter coefficients match clinical band specifications
- **Kalman predictor** state transitions are numerically correct
- **Tremor pipeline** band-power classification produces consistent output
