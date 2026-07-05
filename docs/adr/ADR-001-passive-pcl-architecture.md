# ADR-001 — Passive coherent location as the primary WiFi sensing layer

**Status:** Accepted  
**Date:** 2026-07-05

## Context

Every existing open-source WiFi sensing project (RuView, CSI-Tool, ESP32 CSI) requires the developer to control the transmitter. This creates two hard barriers:

1. You need dedicated hardware (ESP32, Intel 5300 NIC, etc.)
2. You need admin access to the access point to configure CSI extraction

Passive Coherent Location (PCL) — used in military radar since the 1980s — eliminates both barriers. A receiver passively captures signals from an ambient transmitter (a neighbour's router, a public hotspot, any 802.11 beacon) and uses it as the illuminating source. No transmit permission, no hardware control needed.

## Decision

PhysicSense will implement PCL as its primary WiFi sensing modality using:

- **Reference channel:** direct-path signal captured from the ambient transmitter (line-of-sight receive)
- **Surveillance channel:** reflected signal from the sensing volume
- **Cross-correlation:** time-domain cross-correlation gives bistatic delay (range)
- **Slow-time FFT:** applied across successive CPIs to extract Doppler (velocity)
- **Range-Doppler map:** 2-D output per CPI batch

The implementation lives in `core/passive-pcl` and uses `rustfft` for O(N log N) cross-correlation via the frequency-domain approach.

## Consequences

**Positive:**
- Zero dedicated hardware required — any 802.11 monitor-mode capable adapter works
- Legal in most jurisdictions (passive receive only, no transmission)
- Novel in open source — no prior implementation exists
- No CSI firmware patches required

**Negative:**
- Sensitivity is lower than active systems (no transmit power control)
- Ambient signal quality is environment-dependent
- Requires a dual-antenna or dual-channel receive setup for clean reference/surveillance separation
- Browser deployment requires a native helper for raw packet capture (WebExtension bridge)

## Alternatives considered

- **Active CSI (ESP32):** High quality data but requires hardware; not novel.
- **RSSI-only:** Too coarse for vital signs or tremor detection.
- **FMCW radar (60 GHz):** Excellent but $15–$100 hardware; separate from WiFi sensing goal.
