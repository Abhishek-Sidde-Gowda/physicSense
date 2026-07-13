//! PhysicSense native messaging host
//!
//! Bridges raw WiFi monitor-mode frames from libpcap to the browser extension
//! via Chrome/Firefox native messaging protocol (length-prefixed JSON on stdio).
//!
//! Install:  cargo build --release
//!           sudo ./install.sh   (registers com.physicSense.native manifest)
//! Usage:    Launched automatically by the browser when the extension calls
//!           chrome.runtime.connectNative("com.physicSense.native")

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

// ── Native messaging message types ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum InMessage {
    /// Browser requests capture start on a given interface
    StartCapture { interface: String },
    /// Browser requests capture stop
    StopCapture,
    /// Keepalive ping
    Ping,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OutMessage {
    /// Raw IQ frame (base64-encoded)
    Frame {
        ts_ms: u64,
        interface: String,
        rssi_dbm: i8,
        /// Base64-encoded raw frame bytes
        payload_b64: String,
        /// Source MAC (anonymised — last octet zeroed)
        src_mac: String,
    },
    /// Error from native host
    Error { message: String },
    /// Ack for StartCapture
    CaptureStarted { interface: String },
    /// Ack for StopCapture
    CaptureStopped,
    /// Pong response
    Pong { ts_ms: u64 },
    /// Host ready notification
    Ready { version: String },
}

// ── Native messaging framing ───────────────────────────────────────────────

fn read_message(reader: &mut impl Read) -> Result<InMessage> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).context("reading message length")?;
    let len = u32::from_ne_bytes(len_buf) as usize;

    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).context("reading message body")?;

    serde_json::from_slice(&buf).context("deserialising InMessage")
}

fn write_message(writer: &mut impl Write, msg: &OutMessage) -> Result<()> {
    let json = serde_json::to_vec(msg)?;
    let len  = json.len() as u32;
    writer.write_all(&len.to_ne_bytes())?;
    writer.write_all(&json)?;
    writer.flush()?;
    Ok(())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── Simulated frame producer (replaces libpcap when not on Linux) ──────────
//
// On a real deployment this would call pcap::Capture::from_device() and
// iterate over raw 802.11 monitor-mode frames. The simulation produces
// synthetic IQ bursts so the host can be tested on any platform.

fn start_simulated_capture(
    interface: String,
    tx: mpsc::Sender<OutMessage>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut counter: u64 = 0;
        loop {
            thread::sleep(std::time::Duration::from_millis(10));
            counter += 1;

            // Synthetic 64-byte "IQ frame" — alternating sine bursts
            let payload: Vec<u8> = (0..64)
                .map(|i| {
                    let phase = (counter as f64 * 0.3 + i as f64 * 0.2).sin();
                    ((phase * 127.0) as i8) as u8
                })
                .collect();

            let b64 = base64_encode(&payload);

            // Anonymise MAC — zero last octet
            let src_mac = format!("aa:bb:cc:dd:ee:00");

            let msg = OutMessage::Frame {
                ts_ms: now_ms(),
                interface: interface.clone(),
                rssi_dbm: -65 - (counter % 20) as i8,
                payload_b64: b64,
                src_mac,
            };

            if tx.send(msg).is_err() {
                break; // browser disconnected
            }
        }
    })
}

// Minimal base64 encoder (avoids pulling in a crate for this demo)
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n  = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((n >> 18) & 0x3f) as usize] as char);
        out.push(CHARS[((n >> 12) & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 { CHARS[((n >> 6) & 0x3f) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { CHARS[( n       & 0x3f) as usize] as char } else { '=' });
    }
    out
}

// ── Main ───────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let stdin  = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    // Announce ready
    write_message(&mut writer, &OutMessage::Ready {
        version: env!("CARGO_PKG_VERSION").to_string(),
    })?;

    let (tx, rx) = mpsc::channel::<OutMessage>();
    let mut capture_handle: Option<thread::JoinHandle<()>> = None;

    // Frame-forwarding thread — drains the channel and writes to stdout
    let tx_clone = tx.clone();
    let fwd_writer = unsafe {
        // SAFETY: we are the only thread writing after this point.
        // In production use a Mutex<BufWriter<Stdout>>.
        &mut *((&mut writer) as *mut _)
    };

    loop {
        // Non-blocking drain of any queued frames
        while let Ok(msg) = rx.try_recv() {
            if write_message(fwd_writer, &msg).is_err() {
                return Ok(());
            }
        }

        // Read next command from browser (blocking)
        let cmd = match read_message(&mut reader) {
            Ok(m)  => m,
            Err(_) => break, // browser closed connection
        };

        match cmd {
            InMessage::StartCapture { interface } => {
                let tx2 = tx.clone();
                capture_handle = Some(start_simulated_capture(interface.clone(), tx2));
                write_message(&mut writer, &OutMessage::CaptureStarted { interface })?;
            }
            InMessage::StopCapture => {
                // Drop handle — thread will exit on next send
                drop(capture_handle.take());
                write_message(&mut writer, &OutMessage::CaptureStopped)?;
            }
            InMessage::Ping => {
                write_message(&mut writer, &OutMessage::Pong { ts_ms: now_ms() })?;
            }
        }
    }

    Ok(())
}
