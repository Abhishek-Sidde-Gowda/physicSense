# PhysicSense native messaging host

The native host is a small daemon that opens a WiFi adapter in monitor mode
and streams raw 802.11 IQ frames to the browser extension over stdin/stdout.

## Installation

```bash
# macOS (requires airport utility or libpcap)
make install-macos

# Linux (requires libpcap + nl80211)
make install-linux
```

After building, copy `com.physicSense.native.json` to:

- **macOS Chrome:** `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/`
- **macOS Firefox:** `~/Library/Application Support/Mozilla/NativeMessagingHosts/`
- **Linux Chrome:** `~/.config/google-chrome/NativeMessagingHosts/`
- **Linux Firefox:** `~/.mozilla/native-messaging-hosts/`

Replace `YOUR_EXTENSION_ID_HERE` in the JSON with your actual extension ID
(visible in `chrome://extensions` after loading unpacked).

## Message format

Each message is a JSON object prefixed by a 4-byte little-endian length (Chrome
native messaging protocol):

```json
{
  "timestamp_us": 1720000000000000,
  "channel": 6,
  "rssi_dbm": -55,
  "payload_b64": "<base64 encoded float32 IQ samples, interleaved I Q I Q ...>"
}
```

## Building the daemon (Rust)

The daemon source lives in `core/passive-pcl` — the `pcl-capture` binary target
(to be added in a future commit) wraps libpcap and outputs frames in the format above.
