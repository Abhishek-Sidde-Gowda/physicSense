# navigator.physicSense — Web API Specification

**Status:** Draft v0.1  
**Author:** PhysicSense Project  
**Date:** 2026-07-05

---

## Abstract

This specification defines `navigator.physicSense`, a browser API for passive
multi-modal ambient sensing. It unifies three sensing modalities — passive WiFi
bistatic radar, acoustic FMCW ultrasound, and neuromotor biomarker extraction —
behind a single permission-gated interface, following the patterns established
by `navigator.mediaDevices` and `navigator.geolocation`.

No dedicated hardware is required. The API uses the device's existing WiFi
adapter (via a WebExtension bridge for raw frame access), speaker, and
microphone.

---

## Permission Model

```
"physicSense" permission — gated behind an explicit user gesture.
Subdivisions:
  "physicSense.wifi"       — passive WiFi frame capture (requires extension)
  "physicSense.acoustic"   — speaker emit + mic capture (WebAudio)
  "physicSense.neuromotor" — derived biomarker output (no extra hardware)
```

Each sub-permission follows the same pattern as `"camera"` and `"microphone"`:
user must grant it once per origin; revocable at any time from browser settings.

---

## Interface

```webidl
partial interface Navigator {
  [SameObject] readonly attribute PhysicSense physicSense;
};

interface PhysicSense : EventTarget {
  Promise<PhysicSenseSession> requestSession(PhysicSenseSessionInit init);
  Promise<PermissionState>    queryPermission(PhysicSensePermissionDescriptor desc);
};

dictionary PhysicSenseSessionInit {
  sequence<PhysicSenseModality> modalities;   // required
  float                         sampleRate;   // default 44100 (acoustic)
  unsigned long                 wifiCpiSize;  // default 512 (WiFi CPI samples)
};

enum PhysicSenseModality { "wifi", "acoustic", "neuromotor" };

interface PhysicSenseSession : EventTarget {
  attribute EventHandler onframe;       // fires PhysicSenseFrameEvent per CPI
  attribute EventHandler onerror;

  undefined start();
  undefined stop();
  readonly attribute boolean active;
};

interface PhysicSenseFrameEvent : Event {
  readonly attribute PhysicSenseFrame frame;
};

interface PhysicSenseFrame {
  readonly attribute DOMHighResTimeStamp timestamp;

  // WiFi PCL output (null if "wifi" not in modalities)
  readonly attribute Float32Array? rangeDopplerMap;   // range_bins × doppler_bins, row-major
  readonly attribute float?        velocityMs;        // dominant target radial velocity

  // Acoustic FMCW output (null if "acoustic" not in modalities)
  readonly attribute float?        rangeM;            // dominant target range in metres
  readonly attribute Float32Array? position2d;        // [x, y] in metres, null if <3 mics

  // Neuromotor output (null if "neuromotor" not in modalities)
  readonly attribute PhysicSenseNeuromotor? neuromotor;
};

interface PhysicSenseNeuromotor {
  readonly attribute float        tremorDominantHz;   // 0 if no tremor detected
  readonly attribute DOMString    tremorClass;        // "none"|"physiological"|"essential"|"parkinsonian"|"indeterminate"
  readonly attribute float        gaitCadenceSpm;     // steps per minute
  readonly attribute float        gaitAsymmetryIndex; // 0–1
  readonly attribute float        updrsProxyScore;    // 0–8, screening only
  readonly attribute boolean      flagForReview;      // true if score >= 3.0
};

dictionary PhysicSensePermissionDescriptor {
  required DOMString name;  // "physicSense.wifi" | "physicSense.acoustic" | "physicSense.neuromotor"
};
```

---

## Usage Example

```js
// Request a session with all three modalities
const session = await navigator.physicSense.requestSession({
  modalities: ["wifi", "acoustic", "neuromotor"],
  sampleRate: 44100,
  wifiCpiSize: 512,
});

session.onframe = (event) => {
  const { frame } = event;

  console.log("range:", frame.rangeM, "m");
  console.log("velocity:", frame.velocityMs, "m/s");

  if (frame.neuromotor?.flagForReview) {
    console.warn("UPDRS proxy score:", frame.neuromotor.updrsProxyScore);
  }
};

session.onerror = (event) => console.error(event);

session.start();

// Later:
session.stop();
```

---

## WiFi Bridge Protocol (WebExtension ↔ Page)

Raw WiFi frame capture is not available to web pages directly. The polyfill
uses a WebExtension background script that:

1. Spawns a native messaging host (`physicSense-native`) that opens the WiFi
   adapter in monitor mode.
2. Sends raw 802.11 frames as `ArrayBuffer` messages over `chrome.runtime.connect`.
3. The content script exposes these as the `wifi` modality data.

Message format (native host → extension → page):

```json
{
  "type": "wifi_frame",
  "timestamp_us": 1720000000000000,
  "channel": 6,
  "rssi_dbm": -55,
  "payload_b64": "<base64 encoded IQ samples>"
}
```

---

## Privacy & Security

- **No raw IQ data leaves the page.** Only derived outputs (range, velocity,
  neuromotor scores) are surfaced. The raw WiFi frames are processed inside the
  WASM module and discarded.
- **Neuromotor scores are not transmitted.** The API produces local output only;
  no data is sent to any server by the polyfill itself.
- **Acoustic emit is audible disclosure.** User agents MUST display a visible
  indicator (analogous to the camera/mic indicator light) when the acoustic
  modality is active.
- **Permission revocation** immediately terminates any active session.

---

## Relationship to Existing APIs

| This API | Analogous existing API |
|---|---|
| `navigator.physicSense.requestSession()` | `navigator.mediaDevices.getUserMedia()` |
| `PhysicSenseSession.onframe` | `RTCPeerConnection` track events |
| WiFi bridge via WebExtension native messaging | Similar to `chrome.usb` bridge for WebHID |
