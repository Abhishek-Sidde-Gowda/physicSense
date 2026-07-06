/**
 * 2-D Kalman tracker — assigns persistent person IDs across zones.
 *
 * Each track holds a simple constant-velocity Kalman state:
 *   state  = [x, y, vx, vy]
 *   obs    = [x, y]
 *
 * New detections are matched to existing tracks by nearest Mahalanobis
 * distance; unmatched detections spawn new tracks; stale tracks are pruned.
 */

const PRUNE_MS   = 4000;   // drop track after 4 s no match
const MAX_DIST   = 3.0;    // metres — beyond this, don't match
const DT         = 0.1;    // 10 Hz update rate

let _nextId = 1;

function makeTrack(x, y, zoneId, nodeId) {
  return {
    id:      _nextId++,
    state:   [x, y, 0, 0],   // x, y, vx, vy
    P:       [1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1],  // 4×4 covariance (flat)
    zoneId,
    nodeId,
    lastSeen: Date.now(),
    trail:   [{ x, y, t: Date.now() }],
  };
}

// ── Kalman predict step ────────────────────────────────────────────────────
function predict(track) {
  const [x, y, vx, vy] = track.state;
  track.state = [x + vx * DT, y + vy * DT, vx, vy];
  // Inflate covariance (process noise Q)
  const q = 0.1;
  for (let i = 0; i < 16; i++) track.P[i] *= 1.02;
  track.P[0]  += q;
  track.P[5]  += q;
  track.P[10] += q * 0.5;
  track.P[15] += q * 0.5;
}

// ── Kalman update step (obs = [x, y]) ─────────────────────────────────────
function update(track, ox, oy) {
  const [x, y, vx, vy] = track.state;
  const P = track.P;
  const R = 0.3;  // measurement noise

  // Innovation
  const ix = ox - x;
  const iy = oy - y;

  // Kalman gain (2×4 simplified, only x/y rows)
  const Kx = P[0] / (P[0] + R);
  const Ky = P[5] / (P[5] + R);

  track.state = [
    x  + Kx * ix,
    y  + Ky * iy,
    vx + (P[8]  / (P[0] + R)) * ix,
    vy + (P[13] / (P[5] + R)) * iy,
  ];

  // Update covariance
  track.P[0]  *= (1 - Kx);
  track.P[5]  *= (1 - Ky);
}

// ── Euclidean distance between track predicted pos and observation ─────────
function dist(track, ox, oy) {
  const dx = track.state[0] - ox;
  const dy = track.state[1] - oy;
  return Math.sqrt(dx * dx + dy * dy);
}

// ── Main tracker class ─────────────────────────────────────────────────────
export class BuildingTracker {
  constructor() {
    this.tracks = [];
  }

  /**
   * Feed a batch of detections from one node.
   * detections: [{ x, y, zoneId, nodeId }]
   * Returns current snapshot: [{ id, x, y, vx, vy, zoneId, nodeId, trail }]
   */
  update(detections) {
    const now = Date.now();

    // Predict all tracks forward
    this.tracks.forEach(predict);

    // Greedy nearest-neighbour assignment
    const assigned = new Set();
    for (const det of detections) {
      let bestTrack = null;
      let bestDist  = MAX_DIST;
      for (const track of this.tracks) {
        if (assigned.has(track.id)) continue;
        const d = dist(track, det.x, det.y);
        if (d < bestDist) { bestDist = d; bestTrack = track; }
      }
      if (bestTrack) {
        update(bestTrack, det.x, det.y);
        bestTrack.zoneId   = det.zoneId;
        bestTrack.nodeId   = det.nodeId;
        bestTrack.lastSeen = now;
        bestTrack.trail.push({ x: bestTrack.state[0], y: bestTrack.state[1], t: now });
        if (bestTrack.trail.length > 40) bestTrack.trail.shift();
        assigned.add(bestTrack.id);
      } else {
        // New track
        const t = makeTrack(det.x, det.y, det.zoneId, det.nodeId);
        this.tracks.push(t);
      }
    }

    // Prune stale tracks
    this.tracks = this.tracks.filter(t => (now - t.lastSeen) < PRUNE_MS);

    return this.snapshot();
  }

  snapshot() {
    return this.tracks.map(t => ({
      id:     t.id,
      x:      t.state[0],
      y:      t.state[1],
      vx:     t.state[2],
      vy:     t.state[3],
      zoneId: t.zoneId,
      nodeId: t.nodeId,
      trail:  t.trail.slice(-20),
    }));
  }

  count() { return this.tracks.length; }
}
