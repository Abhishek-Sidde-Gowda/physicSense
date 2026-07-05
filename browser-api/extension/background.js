'use strict';

/**
 * Background service worker.
 *
 * Lifecycle:
 *  1. Content script sends { type: "physicSense_ping" } from the page.
 *  2. Background receives it, opens a native messaging port to
 *     "com.physicSense.native" host (the monitor-mode capture daemon).
 *  3. Native host sends raw WiFi frames; background relays them to the tab.
 *  4. Content script posts them to the page via window.postMessage.
 */

const NATIVE_HOST = 'com.physicSense.native';

// Tab ID → native port map
const activePorts = new Map();

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (!sender.tab) return;

  switch (message.type) {
    case 'physicSense_ping':
      sendResponse({ type: 'physicSense_pong' });
      break;

    case 'physicSense_start_wifi':
      startNativeCapture(sender.tab.id);
      sendResponse({ ok: true });
      break;

    case 'physicSense_stop_wifi':
      stopNativeCapture(sender.tab.id);
      sendResponse({ ok: true });
      break;
  }

  // Return true to keep the message channel open for async sendResponse
  return true;
});

function startNativeCapture(tabId) {
  if (activePorts.has(tabId)) return;

  let port;
  try {
    port = chrome.runtime.connectNative(NATIVE_HOST);
  } catch (err) {
    console.warn('[PhysicSense] Native host not installed:', err.message);
    chrome.tabs.sendMessage(tabId, {
      type: 'physicSense_native_unavailable',
      reason: err.message,
    });
    return;
  }

  activePorts.set(tabId, port);

  port.onMessage.addListener((nativeFrame) => {
    // Relay raw frame to the content script in the tab
    chrome.tabs.sendMessage(tabId, {
      type:         'physicSense_wifi_frame_relay',
      timestamp_us: nativeFrame.timestamp_us,
      channel:      nativeFrame.channel,
      rssi_dbm:     nativeFrame.rssi_dbm,
      payload_b64:  nativeFrame.payload_b64,
    }).catch(() => {
      // Tab may have closed — clean up
      stopNativeCapture(tabId);
    });
  });

  port.onDisconnect.addListener(() => {
    activePorts.delete(tabId);
    const err = chrome.runtime.lastError;
    if (err) {
      console.warn('[PhysicSense] Native port disconnected:', err.message);
    }
  });
}

function stopNativeCapture(tabId) {
  const port = activePorts.get(tabId);
  if (port) {
    port.disconnect();
    activePorts.delete(tabId);
  }
}

// Clean up when a tab is closed
chrome.tabs.onRemoved.addListener((tabId) => stopNativeCapture(tabId));
