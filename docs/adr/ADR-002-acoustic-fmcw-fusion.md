# ADR-002 — Acoustic FMCW ultrasound as the secondary sensing layer

**Status:** Accepted  
**Date:** 2026-07-05

## Context

WiFi PCL alone has range resolution limited by signal bandwidth (~20 MHz for 802.11n → ~7.5 m). This is insufficient for centimetre-accurate gesture recognition or fine motor tracking. A complementary modality is needed that:

- Requires no additional hardware (uses existing speaker + mic)
- Provides centimetre-level resolution
- Fuses cleanly with the bistatic WiFi data

## Decision

Emit a 18–22 kHz FMCW chirp (4 kHz bandwidth) from the device speaker and capture reflections on the microphone. The 4 kHz ultrasonic bandwidth gives:

```
range_resolution = c / (2 * BW) = 343 / 8000 ≈ 4.3 cm
max_range = c * T_chirp / 2 = 343 * 0.02 / 2 ≈ 3.4 m
```

For multi-mic setups (laptop corners, phone + earbuds), TDOA between mic pairs gives a 2-D position fix via Gauss-Newton least-squares.

The implementation lives in `core/acoustic-dsp`. Browser deployment uses the WebAudio API — no native code required for this layer.

## Consequences

**Positive:**
- 4.3 cm range resolution vs ~7.5 m for WiFi alone
- Works entirely in-browser via WebAudio API
- Multi-mic TDOA gives 2-D position, complementing WiFi Doppler
- 18–22 kHz is inaudible to most adults (above 16 kHz hearing threshold)

**Negative:**
- 18–22 kHz may be audible to children and some pets
- Requires speaker + mic on same device (standard on all laptops/phones)
- Reflections from walls create multipath; needs room impulse response compensation
- Max range ~3.4 m limits to single-room sensing

## Fusion strategy

WiFi PCL gives: coarse range + Doppler (velocity)  
Acoustic FMCW gives: fine range + 2-D position  
Combined: range-Doppler + position → full kinematic state, input to neuromotor biomarker extraction
