'use strict';

/**
 * Content script — runs in the page's context but has access to
 * chrome.runtime. Bridges postMessage (page → extension) and
 * chrome.runtime.sendMessage (extension → background).
 *
 * Security: only relay messages that match our protocol type prefix.
 * Never echo arbitrary page messages to the background.
 */

const ALLOWED_TYPES = new Set([
  'physicSense_ping',
  'physicSense_start_wifi',
  'physicSense_stop_wifi',
]);

// Page → Extension
window.addEventListener('message', async (event) => {
  // Accept messages only from same origin
  if (event.origin !== window.location.origin) return;
  if (!event.data?.type || !ALLOWED_TYPES.has(event.data.type)) return;

  try {
    const response = await chrome.runtime.sendMessage(event.data);
    // Relay response back to page
    window.postMessage(response, window.location.origin);
  } catch (err) {
    // Extension context invalidated (reload) — ignore
  }
});

// Extension → Page (native WiFi frames relayed from background)
chrome.runtime.onMessage.addListener((message) => {
  if (message.type === 'physicSense_wifi_frame_relay') {
    // Rewrite type to what the page's WifiModality listener expects
    window.postMessage(
      { ...message, type: 'physicSense_wifi_frame' },
      window.location.origin
    );
  }

  if (message.type === 'physicSense_native_unavailable') {
    window.postMessage(message, window.location.origin);
  }
});
