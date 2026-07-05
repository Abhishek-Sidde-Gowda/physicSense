# ADR-003 — Federated learning with (ε, δ)-DP via Gaussian mechanism

**Status:** Accepted  
**Date:** 2026-07-05

## Context

PhysicSense nodes collect neuromotor biomarker data that is sensitive by nature
— tremor patterns and gait asymmetry could reveal medical conditions. A single
centralised model trained on pooled data would require raw data to leave each
device, which is unacceptable for a medical-adjacent application.

Federated learning (McMahan et al., 2017) allows nodes to collaboratively train
a shared model without sharing raw data. Each node trains locally and shares
only gradient updates. DP-SGD (Abadi et al., 2016) further ensures that even
gradient updates reveal negligible information about local data.

## Decision

**Federated FedAvg with (ε, δ)-DP Gaussian mechanism:**

1. Each node computes gradients from local sensing data only.
2. Per-sample gradients are clipped to L2 norm `C` (sensitivity bound).
3. Gaussian noise `N(0, σ²C²I)` is added, where `σ = sqrt(2 ln(1.25/δ) T) / ε`.
4. The noisy average gradient is transmitted over a WebRTC data channel.
5. The coordinator aggregates via weighted FedAvg (weight = local sample count).
6. The global update is broadcast back; no node ID is attached to it.

Default privacy budget: **ε = 1.0, δ = 1e-5** (strong privacy, medical setting).
Moderate budget: **ε = 4.0, δ = 1e-5** (better utility for non-medical use).

## Consequences

**Positive:**
- Raw sensing data never leaves the device
- (ε, δ)-DP provides formal, quantifiable privacy guarantees
- FedAvg is robust to partial node participation (any node missing a round is skipped)
- WebRTC peer-to-peer means gradient updates do not pass through any server

**Negative:**
- DP noise reduces model accuracy — especially severe for small node counts
- Each training round requires synchronisation across nodes (latency)
- The moments accountant (privacy cost tracking) is simplified here; production needs the full Rényi DP accountant
- WebRTC requires a signalling server for initial SDP exchange

## Privacy accounting

Privacy cost grows with rounds. The `DifferentialPrivacy::privacy_spent(steps)`
method tracks cumulative (ε, δ) spent. Nodes should stop contributing once
ε_spent exceeds a configured threshold.
