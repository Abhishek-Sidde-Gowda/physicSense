# Contributing to PhysicSense

Thank you for your interest. PhysicSense is an active research project —
contributions, bug reports, and ideas are welcome.

## Getting started

```bash
git clone https://github.com/Abhishek-Sidde-Gowda/physicSense.git
cd physicSense

# Build Rust workspace
cargo build --workspace
cargo test --workspace

# Run benchmarks
node benchmarks/bench.js

# Start dashboard
cd dashboard && npx serve public -p 3000

# Start building hub
cd tracking && npm install && node hub.js
```

## Project structure

```
core/
  passive-pcl/    GCC-PHAT bistatic WiFi radar
  acoustic-dsp/   FMCW chirp + TDOA localiser
  neuromotor/     Tremor classification + gait + UPDRS proxy
fusion/           Attention-based multi-modal fusion (WASM)
federated/        DP-SGD + FedAvg + WebRTC transport
browser-api/      navigator.physicSense polyfill + WebExtension
native-host/      libpcap WiFi bridge (Rust binary)
tracking/         Multi-zone Kalman tracker + WebSocket hub
dashboard/        Three.js live 3-D visualisation
benchmarks/       Deterministic proof scripts + SHA-256 hashes
research/         Clinical validation protocol + UPDRS methodology
docs/             Architecture SVG, ADRs
```

## How to contribute

1. **Fork** the repo and create a branch: `git checkout -b feat/your-feature`
2. Make your changes with clear, focused commits
3. Run `cargo test --workspace` and `node benchmarks/bench.js` — both must pass
4. Open a pull request with a description of what and why

## Code style

- Rust: `cargo clippy` must pass with no warnings
- JavaScript: ES modules, no bundler required
- No dead code, no commented-out blocks

## Research contributions

If you are extending the clinical validation study or adding new neuromotor
biomarkers, please read `research/updrs-proxy-methodology.md` and
`research/clinical-validation-protocol.md` first.

All clinical claims must be clearly marked as **screening only** — not
diagnostic. See the disclaimer in `README.md`.

## Reporting issues

Open a GitHub issue with:
- What you expected to happen
- What actually happened
- Steps to reproduce
- Platform (OS, Node version, Rust version)

## License

By contributing you agree your work is released under the MIT License.
