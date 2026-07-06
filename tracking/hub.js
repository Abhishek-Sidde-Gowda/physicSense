/**
 * PhysicSense Building Hub — WebSocket aggregation server
 *
 * Listens on two ports:
 *   8765  — PhysicSense sensor nodes connect here and send detections
 *   8766  — Dashboard clients connect here and receive building state
 *
 * Run:  node tracking/hub.js
 */

import { WebSocketServer, WebSocket } from 'ws';
import { BuildingTracker } from './tracker.js';

const NODE_PORT      = 8765;
const DASHBOARD_PORT = 8766;
const BROADCAST_HZ   = 10;

// ── Zone definitions (metres, origin = building entrance) ─────────────────
// Matches the floor plan drawn in floorplan.html
export const ZONES = [
  { id: 'z1',  name: 'Entrance Hall',   x: 0,   y: 0,   w: 8,  h: 5  },
  { id: 'z2',  name: 'Corridor A',      x: 0,   y: 5,   w: 20, h: 3  },
  { id: 'z3',  name: 'Room 101',        x: 0,   y: 8,   w: 6,  h: 6  },
  { id: 'z4',  name: 'Room 102',        x: 7,   y: 8,   w: 6,  h: 6  },
  { id: 'z5',  name: 'Room 103',        x: 14,  y: 8,   w: 6,  h: 6  },
  { id: 'z6',  name: 'Corridor B',      x: 0,   y: 14,  w: 20, h: 3  },
  { id: 'z7',  name: 'Room 201',        x: 0,   y: 17,  w: 6,  h: 6  },
  { id: 'z8',  name: 'Room 202',        x: 7,   y: 17,  w: 6,  h: 6  },
  { id: 'z9',  name: 'Room 203',        x: 14,  y: 17,  w: 6,  h: 6  },
  { id: 'z10', name: 'Stairwell',       x: 21,  y: 5,   w: 4,  h: 18 },
];

function zoneForPoint(x, y) {
  for (const z of ZONES) {
    if (x >= z.x && x < z.x + z.w && y >= z.y && y < z.y + z.h) return z.id;
  }
  return 'unknown';
}

// ── State ─────────────────────────────────────────────────────────────────
const tracker    = new BuildingTracker();
const nodeClients = new Map();   // nodeId → ws
const dashClients = new Set();
let   nodeCounter = 0;

// ── Node server ───────────────────────────────────────────────────────────
const nodeServer = new WebSocketServer({ port: NODE_PORT });
nodeServer.on('listening', () => console.log(`[hub] node port  ${NODE_PORT}`));

nodeServer.on('connection', (ws) => {
  const nodeId = `node-${++nodeCounter}`;
  nodeClients.set(nodeId, ws);
  console.log(`[hub] node connected: ${nodeId}`);

  ws.send(JSON.stringify({ type: 'welcome', nodeId, zones: ZONES }));

  ws.on('message', (raw) => {
    let msg;
    try { msg = JSON.parse(raw); } catch { return; }

    if (msg.type === 'detections') {
      // msg.detections = [{ x, y }]  (local coords, metres from node origin)
      // msg.origin     = { x, y }    (node origin in building coords)
      const origin = msg.origin ?? { x: 0, y: 0 };
      const dets = (msg.detections ?? []).map(d => {
        const bx = d.x + origin.x;
        const by = d.y + origin.y;
        return { x: bx, y: by, zoneId: zoneForPoint(bx, by), nodeId };
      });
      tracker.update(dets);
    }
  });

  ws.on('close', () => {
    nodeClients.delete(nodeId);
    console.log(`[hub] node disconnected: ${nodeId}`);
  });
});

// ── Dashboard server ───────────────────────────────────────────────────────
const dashServer = new WebSocketServer({ port: DASHBOARD_PORT });
dashServer.on('listening', () => console.log(`[hub] dash port  ${DASHBOARD_PORT}`));

dashServer.on('connection', (ws) => {
  dashClients.add(ws);
  // Send zones on connect so dashboard can draw the floor plan
  ws.send(JSON.stringify({ type: 'zones', zones: ZONES }));
  ws.on('close', () => dashClients.delete(ws));
});

// ── Broadcast loop ────────────────────────────────────────────────────────
function broadcast() {
  const persons   = tracker.snapshot();
  const occupancy = {};
  ZONES.forEach(z => { occupancy[z.id] = 0; });
  persons.forEach(p => {
    if (occupancy[p.zoneId] !== undefined) occupancy[p.zoneId]++;
  });

  const msg = JSON.stringify({
    type:      'building_state',
    ts:        Date.now(),
    total:     persons.length,
    persons,
    occupancy,
    nodes:     [...nodeClients.keys()],
  });

  dashClients.forEach(ws => {
    if (ws.readyState === WebSocket.OPEN) ws.send(msg);
  });
}

setInterval(broadcast, 1000 / BROADCAST_HZ);

console.log('[hub] PhysicSense building hub started');
