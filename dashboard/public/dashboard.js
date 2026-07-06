import * as THREE from 'three';

// ── WebSocket / demo mode ────────────────────────────────────────────────────
let ws = null;
let demoInterval = null;
let demoT = 0;

// ── 17-keypoint skeleton definition (COCO format) ────────────────────────────
const BONES = [
  [0,1],[0,2],[1,3],[2,4],          // head
  [5,6],[5,7],[7,9],[6,8],[8,10],   // arms
  [5,11],[6,12],[11,12],            // torso
  [11,13],[13,15],[12,14],[14,16],  // legs
];
const JOINT_COLORS = {
  head:  0x60a5fa,
  torso: 0x22c55e,
  arm:   0xf59e0b,
  leg:   0xa78bfa,
};

// ── Three.js scene setup ─────────────────────────────────────────────────────
const canvas  = document.getElementById('three-canvas');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
renderer.setPixelRatio(window.devicePixelRatio);
renderer.setClearColor(0x0f1117, 1);

const scene  = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(55, 1, 0.01, 100);
camera.position.set(0, 1.2, 3.5);
camera.lookAt(0, 0.9, 0);

// Lighting
scene.add(new THREE.AmbientLight(0xffffff, 0.4));
const dirLight = new THREE.DirectionalLight(0x60a5fa, 1.2);
dirLight.position.set(2, 4, 3);
scene.add(dirLight);
const rimLight = new THREE.DirectionalLight(0x3b82f6, 0.5);
rimLight.position.set(-2, 1, -2);
scene.add(rimLight);

// Floor grid
const gridHelper = new THREE.GridHelper(6, 12, 0x2a3350, 0x1e2535);
scene.add(gridHelper);

// ── Skeleton joints & bones ───────────────────────────────────────────────────
const jointMeshes = [];
const boneMeshes  = [];

for (let i = 0; i < 17; i++) {
  const color = i < 5 ? JOINT_COLORS.head
              : i < 11 ? JOINT_COLORS.arm
              : i < 13 ? JOINT_COLORS.torso
              : JOINT_COLORS.leg;
  const geo  = new THREE.SphereGeometry(0.028, 10, 10);
  const mat  = new THREE.MeshPhongMaterial({ color, emissive: color, emissiveIntensity: 0.3 });
  const mesh = new THREE.Mesh(geo, mat);
  mesh.visible = false;
  scene.add(mesh);
  jointMeshes.push(mesh);
}

for (let i = 0; i < BONES.length; i++) {
  const mat  = new THREE.LineBasicMaterial({ color: 0x3b82f6, linewidth: 2, opacity: 0.8, transparent: true });
  const geo  = new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(), new THREE.Vector3()]);
  const line = new THREE.Line(geo, mat);
  line.visible = false;
  scene.add(line);
  boneMeshes.push(line);
}

// ── Presence sphere (pulses when person detected) ────────────────────────────
const presenceGeo = new THREE.SphereGeometry(0.12, 16, 16);
const presenceMat = new THREE.MeshPhongMaterial({
  color: 0x3b82f6, emissive: 0x3b82f6, emissiveIntensity: 0.6,
  transparent: true, opacity: 0.7, wireframe: false,
});
const presenceSphere = new THREE.Mesh(presenceGeo, presenceMat);
presenceSphere.position.set(0, 0.9, 0);
scene.add(presenceSphere);

// Range-Doppler heatmap plane
const rdCanvas = document.createElement('canvas');
rdCanvas.width  = 64;
rdCanvas.height = 8;
const rdCtx     = rdCanvas.getContext('2d');
const rdTex     = new THREE.CanvasTexture(rdCanvas);
const rdPlane   = new THREE.Mesh(
  new THREE.PlaneGeometry(1.6, 0.2),
  new THREE.MeshBasicMaterial({ map: rdTex, transparent: true, opacity: 0.85 })
);
rdPlane.rotation.x = -Math.PI / 2;
rdPlane.position.set(-1.8, 0.01, 0.6);
scene.add(rdPlane);

// ── Resize handler ───────────────────────────────────────────────────────────
function resize() {
  const w = canvas.clientWidth;
  const h = canvas.clientHeight;
  if (renderer.domElement.width !== w || renderer.domElement.height !== h) {
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
  }
}

// ── Smooth camera orbit ───────────────────────────────────────────────────────
let orbitAngle = 0;

// ── Timeline canvas setup ────────────────────────────────────────────────────
const tlCanvas = document.getElementById('timeline-canvas');
const tlCtx    = tlCanvas.getContext('2d');
const TL_MAX   = 600; // 60 s at 10 Hz
const tlData   = {
  parkinsonian: new Float32Array(TL_MAX),
  essential:    new Float32Array(TL_MAX),
  physiological:new Float32Array(TL_MAX),
  updrs:        new Float32Array(TL_MAX),
};
let tlHead = 0;

function drawTimeline() {
  const w = tlCanvas.offsetWidth;
  const h = tlCanvas.offsetHeight;
  tlCanvas.width  = w;
  tlCanvas.height = h;

  tlCtx.clearRect(0, 0, w, h);
  tlCtx.fillStyle = '#161b27';
  tlCtx.fillRect(0, 0, w, h);

  // Grid lines
  tlCtx.strokeStyle = '#2a3350';
  tlCtx.lineWidth = 0.5;
  for (let y = 0; y <= 4; y++) {
    const yy = (y / 4) * h;
    tlCtx.beginPath(); tlCtx.moveTo(0, yy); tlCtx.lineTo(w, yy); tlCtx.stroke();
  }

  const series = [
    { key: 'parkinsonian',  color: '#ef4444' },
    { key: 'essential',     color: '#f59e0b' },
    { key: 'physiological', color: '#3b82f6' },
  ];

  series.forEach(({ key, color }) => {
    const data = tlData[key];
    tlCtx.beginPath();
    tlCtx.strokeStyle = color;
    tlCtx.lineWidth = 1.5;
    for (let i = 0; i < TL_MAX; i++) {
      const idx = (tlHead + i) % TL_MAX;
      const x   = (i / TL_MAX) * w;
      const y   = h - data[idx] * h * 0.85 - 2;
      i === 0 ? tlCtx.moveTo(x, y) : tlCtx.lineTo(x, y);
    }
    tlCtx.stroke();
  });

  // Legend
  const labels = [
    { color: '#ef4444', text: 'Parkinsonian' },
    { color: '#f59e0b', text: 'Essential' },
    { color: '#3b82f6', text: 'Physiological' },
  ];
  labels.forEach(({ color, text }, i) => {
    tlCtx.fillStyle = color;
    tlCtx.fillRect(8 + i * 90, 6, 10, 4);
    tlCtx.fillStyle = '#64748b';
    tlCtx.font = '9px system-ui';
    tlCtx.fillText(text, 22 + i * 90, 13);
  });
}

// ── Update skeleton from keypoints array ─────────────────────────────────────
function updateSkeleton(keypoints) {
  if (!keypoints || keypoints.length < 17) {
    jointMeshes.forEach(m => m.visible = false);
    boneMeshes.forEach(m => m.visible = false);
    return;
  }
  keypoints.forEach((kp, i) => {
    jointMeshes[i].position.set(kp.x, kp.y, kp.z);
    jointMeshes[i].visible = true;
  });
  BONES.forEach(([a, b], i) => {
    const pa = jointMeshes[a].position;
    const pb = jointMeshes[b].position;
    const positions = boneMeshes[i].geometry.attributes.position;
    positions.setXYZ(0, pa.x, pa.y, pa.z);
    positions.setXYZ(1, pb.x, pb.y, pb.z);
    positions.needsUpdate = true;
    boneMeshes[i].visible = true;
  });
}

// ── Update range-Doppler heatmap ──────────────────────────────────────────────
function updateRangeDoppler(flat) {
  if (!flat || flat.length < 64 * 8) return;
  const imgData = rdCtx.createImageData(64, 8);
  let maxVal = 0;
  for (let i = 0; i < flat.length; i++) maxVal = Math.max(maxVal, flat[i]);
  if (maxVal < 1e-6) maxVal = 1;
  for (let i = 0; i < flat.length; i++) {
    const norm = flat[i] / maxVal;
    const r = Math.min(255, norm * 600);
    const g = Math.min(255, norm * 200);
    const b = Math.min(255, (1 - norm) * 200 + norm * 50);
    imgData.data[i * 4]     = r;
    imgData.data[i * 4 + 1] = g;
    imgData.data[i * 4 + 2] = b;
    imgData.data[i * 4 + 3] = 220;
  }
  rdCtx.putImageData(imgData, 0, 0);
  rdTex.needsUpdate = true;
}

// ── Update all UI panels ──────────────────────────────────────────────────────
const TREMOR_BADGE_CLASS = {
  none:          'badge-none',
  physiological: 'badge-physiological',
  essential:     'badge-essential',
  parkinsonian:  'badge-parkinsonian',
  indeterminate: 'badge-indeterminate',
};

function updateUI(frame) {
  // Vitals
  const br = frame.breathingRate ?? 0;
  const hr = frame.heartRate     ?? 0;
  const rm = frame.rangeM        ?? 0;
  const vl = Math.abs(frame.velocityMs ?? 0);

  setText('v-breath', br.toFixed(1));
  setText('v-hr',     hr.toFixed(0));
  setText('v-range',  rm.toFixed(2));
  setText('v-vel',    vl.toFixed(2));
  setWidth('bar-breath', (br / 30) * 100);
  setWidth('bar-hr',     (hr / 120) * 100);
  setWidth('bar-range',  (rm / 4)   * 100);
  setWidth('bar-vel',    (vl / 2)   * 100);

  // Scene overlay
  setText('tag-range', rm.toFixed(2));
  setText('tag-vel',   (frame.velocityMs ?? 0).toFixed(2));
  if (frame.position2d) {
    setText('tag-pos', `(${frame.position2d[0].toFixed(1)}, ${frame.position2d[1].toFixed(1)})`);
  }

  // Neuromotor
  const nm = frame.neuromotor;
  if (nm) {
    const badge = document.getElementById('tremor-badge');
    badge.textContent = capitalize(nm.tremorClass);
    badge.className   = `tremor-badge ${TREMOR_BADGE_CLASS[nm.tremorClass] ?? 'badge-none'}`;
    setText('tremor-hz', `${nm.tremorDominantHz.toFixed(1)} Hz`);

    setWidth('band-park', nm.parkinsonian_power * 100);
    setWidth('band-ess',  nm.essential_power    * 100);
    setWidth('band-phys', nm.physiological_power * 100);
    setText('pct-park', `${(nm.parkinsonian_power * 100).toFixed(0)}%`);
    setText('pct-ess',  `${(nm.essential_power    * 100).toFixed(0)}%`);
    setText('pct-phys', `${(nm.physiological_power * 100).toFixed(0)}%`);

    const score = nm.updrsProxyScore;
    setText('updrs-score',  score.toFixed(1));
    setText('updrs-tremor', nm.tremorSubscore?.toFixed(1) ?? '—');
    setText('updrs-gait',   nm.gaitSubscore?.toFixed(1)   ?? '—');
    const scoreEl = document.getElementById('updrs-score');
    scoreEl.style.color = score < 3 ? '#22c55e' : score < 6 ? '#f59e0b' : '#ef4444';
    document.getElementById('updrs-marker').style.left = `${(score / 8) * 100}%`;
    document.getElementById('updrs-flag').classList.toggle('visible', nm.flagForReview);

    // Gait
    setText('g-cadence', nm.gaitCadenceSpm?.toFixed(0) ?? '—');
    setText('g-asym',    nm.gaitAsymmetryIndex?.toFixed(2) ?? '—');
    setText('g-cv',      nm.gaitStrideCv?.toFixed(1) ?? '—');
    setText('g-fog',     nm.fogRisk?.toFixed(2) ?? '—');

    // Timeline push
    tlData.parkinsonian[tlHead]  = nm.parkinsonian_power;
    tlData.essential[tlHead]     = nm.essential_power;
    tlData.physiological[tlHead] = nm.physiological_power;
    tlData.updrs[tlHead]         = score / 8;
    tlHead = (tlHead + 1) % TL_MAX;
  }

  // Skeleton
  if (frame.keypoints) updateSkeleton(frame.keypoints);

  // Range-Doppler
  if (frame.rangeDopplerMap) updateRangeDoppler(frame.rangeDopplerMap);

  // Presence sphere pulse
  presenceSphere.visible = (rm > 0.1);
  if (frame.position2d) {
    presenceSphere.position.set(frame.position2d[0] - 0.5, 0.9, frame.position2d[1] - 0.5);
  }
}

// ── Demo data generator ───────────────────────────────────────────────────────
function demoKeypoints(t) {
  const breathPhase = Math.sin(t * 0.25) * 0.015;
  const sway = Math.sin(t * 0.4) * 0.02;
  const kps = [
    { x: sway,       y: 1.74 + breathPhase, z: 0 },  // 0 nose
    { x: -0.06+sway, y: 1.72 + breathPhase, z: 0 },  // 1 left eye
    { x:  0.06+sway, y: 1.72 + breathPhase, z: 0 },  // 2 right eye
    { x: -0.09+sway, y: 1.68 + breathPhase, z: 0 },  // 3 left ear
    { x:  0.09+sway, y: 1.68 + breathPhase, z: 0 },  // 4 right ear
    { x: -0.18+sway, y: 1.48 + breathPhase, z: 0 },  // 5 left shoulder
    { x:  0.18+sway, y: 1.48 + breathPhase, z: 0 },  // 6 right shoulder
    { x: -0.28+sway, y: 1.18 + Math.sin(t*0.9)*0.04, z: 0.04 },  // 7 left elbow
    { x:  0.28+sway, y: 1.18 + Math.sin(t*0.9+1)*0.04, z: 0.04 }, // 8 right elbow
    { x: -0.32+sway, y: 0.88 + Math.sin(t*0.9)*0.06, z: 0.08 },  // 9 left wrist
    { x:  0.32+sway, y: 0.88 + Math.sin(t*0.9+1)*0.06, z: 0.08 }, // 10 right wrist
    { x: -0.12+sway, y: 0.90 + breathPhase*0.5, z: 0 }, // 11 left hip
    { x:  0.12+sway, y: 0.90 + breathPhase*0.5, z: 0 }, // 12 right hip
    { x: -0.13+sway, y: 0.48 + Math.sin(t*0.5)*0.01, z: 0.02 }, // 13 left knee
    { x:  0.13+sway, y: 0.48 + Math.sin(t*0.5+0.5)*0.01, z: 0.02 }, // 14 right knee
    { x: -0.12+sway, y: 0.04, z: 0.01 }, // 15 left ankle
    { x:  0.12+sway, y: 0.04, z: 0.01 }, // 16 right ankle
  ];
  return kps;
}

function generateDemoFrame(t) {
  const breathing = 15 + Math.sin(t * 0.13) * 3;
  const heartRate = 68 + Math.sin(t * 0.07) * 5;
  const range     = 1.4 + Math.sin(t * 0.11) * 0.3;
  const velocity  = Math.sin(t * 0.23) * 0.15;

  // Tremor: slowly shift between classes over the demo
  const cycle = (t / 60) % 1;
  let tremorClass = 'none';
  let park = 0.05, ess = 0.05, phys = 0.1;
  let domHz = 0;
  if (cycle < 0.3) {
    tremorClass = 'physiological'; phys = 0.6 + Math.random()*0.1; domHz = 10 + Math.random();
  } else if (cycle < 0.55) {
    tremorClass = 'essential'; ess = 0.55 + Math.random()*0.1; domHz = 7 + Math.random();
  } else if (cycle < 0.8) {
    tremorClass = 'parkinsonian'; park = 0.65 + Math.random()*0.1; domHz = 4.5 + Math.random()*0.5;
  } else {
    tremorClass = 'none'; domHz = 0;
  }

  const tremorSubscore = { none: 0, physiological: 0.5, essential: 1.5, parkinsonian: 2.8 }[tremorClass];
  const gaitAsymmetryIndex = tremorClass === 'parkinsonian' ? 0.32 + Math.random()*0.05 : 0.04 + Math.random()*0.02;
  const gaitStrideCv = tremorClass === 'parkinsonian' ? 18 + Math.random()*4 : 3 + Math.random();
  const fogRisk = tremorClass === 'parkinsonian' ? 0.4 + Math.random()*0.1 : 0.05;
  const gaitSubscore = Math.min((gaitAsymmetryIndex*1.5) + (gaitStrideCv/30*1.5) + fogRisk, 4);
  const updrsProxyScore = Math.min(tremorSubscore + gaitSubscore, 8);

  // Fake range-Doppler map
  const rdMap = new Float32Array(64 * 8);
  const peakR = Math.floor((range / 4) * 64);
  const peakD = 4 + Math.round(velocity * 10);
  for (let r = 0; r < 64; r++) {
    for (let d = 0; d < 8; d++) {
      const dr = Math.abs(r - peakR);
      const dd = Math.abs(d - peakD);
      rdMap[r * 8 + d] = Math.exp(-(dr*dr/20 + dd*dd/3)) * (0.8 + Math.random()*0.2);
    }
  }

  return {
    breathingRate: breathing,
    heartRate,
    rangeM: range,
    velocityMs: velocity,
    position2d: [range * Math.cos(t * 0.08), range * Math.sin(t * 0.08) * 0.3],
    rangeDopplerMap: rdMap,
    keypoints: demoKeypoints(t),
    neuromotor: {
      tremorClass, tremorDominantHz: domHz,
      parkinsonian_power: park, essential_power: ess, physiological_power: phys,
      updrsProxyScore, tremorSubscore, gaitSubscore,
      gaitCadenceSpm: 108 + Math.random()*4,
      gaitAsymmetryIndex, gaitStrideCv, fogRisk,
      flagForReview: updrsProxyScore >= 3.0,
    },
  };
}

function startDemo() {
  stopDemo();
  demoT = 0;
  demoInterval = setInterval(() => {
    demoT += 0.1;
    updateUI(generateDemoFrame(demoT));
    drawTimeline();
  }, 100);
}

function stopDemo() {
  if (demoInterval) { clearInterval(demoInterval); demoInterval = null; }
}

// ── WebSocket connection ───────────────────────────────────────────────────────
function connectWS(url) {
  if (ws) ws.close();
  ws = new WebSocket(url);
  ws.onopen = () => {
    stopDemo();
    document.getElementById('ws-dot').className   = 'ws-dot connected';
    document.getElementById('ws-label').textContent = 'live data';
    document.getElementById('mode-pill').textContent = 'LIVE';
    document.getElementById('mode-pill').className   = 'pill pill-live';
  };
  ws.onmessage = (e) => {
    try { updateUI(JSON.parse(e.data)); drawTimeline(); } catch {}
  };
  ws.onclose = () => {
    document.getElementById('ws-dot').className   = 'ws-dot demo';
    document.getElementById('ws-label').textContent = 'disconnected — demo mode';
    document.getElementById('mode-pill').textContent = 'DEMO MODE';
    document.getElementById('mode-pill').className   = 'pill pill-demo';
    startDemo();
  };
  ws.onerror = () => ws.close();
}

// ── Button handlers ───────────────────────────────────────────────────────────
document.getElementById('btn-connect').addEventListener('click', () => {
  const url = prompt('WebSocket URL', 'ws://localhost:8765');
  if (url) connectWS(url);
});
document.getElementById('btn-demo').addEventListener('click', () => {
  if (ws) { ws.close(); ws = null; }
  startDemo();
});

// ── Render loop ───────────────────────────────────────────────────────────────
let frame = 0;
function animate() {
  requestAnimationFrame(animate);
  resize();
  frame++;

  // Slow camera orbit
  orbitAngle += 0.0008;
  camera.position.x = Math.sin(orbitAngle) * 3.5;
  camera.position.z = Math.cos(orbitAngle) * 3.5;
  camera.lookAt(0, 0.9, 0);

  // Presence sphere pulse
  const pulse = 1 + Math.sin(frame * 0.08) * 0.08;
  presenceSphere.scale.setScalar(pulse);

  renderer.render(scene, camera);
}

// ── Init ─────────────────────────────────────────────────────────────────────
animate();
startDemo();

// ── Helpers ──────────────────────────────────────────────────────────────────
function setText(id, val) {
  const el = document.getElementById(id);
  if (el) el.textContent = val;
}
function setWidth(id, pct) {
  const el = document.getElementById(id);
  if (el) el.style.width = `${Math.min(100, Math.max(0, pct))}%`;
}
function capitalize(s) {
  return s ? s.charAt(0).toUpperCase() + s.slice(1) : s;
}
