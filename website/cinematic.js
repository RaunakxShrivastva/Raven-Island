/* ─────────────────────────────────────────────────────────
   cinematic.js — Raven Notch 3D Laptop Animation Sequence
   ─────────────────────────────────────────────────────────
   Timeline (seconds from page load):
     0.0  → Elements fade in: laptop closed on rock
     0.8  → Rock starts sinking
     1.2  → Laptop floats upward
     2.6  → Lid opens
     3.8  → Lid fully open, screen lights up
     4.0  → Raven logo glows
     5.4  → Logo shatters (particle canvas)
     6.0  → Desktop phase begins
     6.2  → Notch drops in from top
     6.8  → Tagline types: "Dynamic Notch for Windows"
     8.0  → CTAs fade in
     8.5  → Notch starts cycling panels
   ───────────────────────────────────────────────────────── */

'use strict';

// ── Link cinematic CSS ──────────────────────────────────
(function() {
  const link = document.createElement('link');
  link.rel = 'stylesheet';
  link.href = 'cinematic.css';
  document.head.appendChild(link);
})();

// ── Utility: promise-based wait ─────────────────────────
function wait(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

// ── Live clock ──────────────────────────────────────────
function formatTime(d) {
  let h = d.getHours(), m = String(d.getMinutes()).padStart(2, '0');
  const ampm = h >= 12 ? 'PM' : 'AM';
  h = h % 12 || 12;
  return { str: `${h}:${m} ${ampm}`, hm: `${h}:${m}`, ampm };
}
function formatDate(d) {
  const M = ['JAN','FEB','MAR','APR','MAY','JUN','JUL','AUG','SEP','OCT','NOV','DEC'];
  return `${M[d.getMonth()]} ${d.getDate()}`;
}
function updateLiveClock() {
  const now = new Date();
  const { str, hm } = formatTime(now);
  const dateStr = formatDate(now);
  const rtime = document.getElementById('c-rtime');
  const rdate = document.getElementById('c-rdate');
  const clockEl = document.getElementById('c-rclock-time');
  const tray   = document.getElementById('c-tray');
  if (rtime)  rtime.textContent = str;
  if (rdate)  rdate.textContent = dateStr;
  if (clockEl) clockEl.textContent = hm;
  if (tray)   tray.textContent  = hm;
}
updateLiveClock();
setInterval(updateLiveClock, 15000);

// ── Particle shatter ────────────────────────────────────
function shatterLogo() {
  const canvas = document.getElementById('c-canvas');
  if (!canvas) return;
  const ctx = canvas.getContext('2d');
  canvas.width  = canvas.offsetWidth  || 484;
  canvas.height = canvas.offsetHeight || 292;

  const cx = canvas.width  / 2;
  const cy = canvas.height / 2;
  const N  = 140;
  const colors = ['#ffffff','#d8b4fe','#a78bfa','#7c3aed','#22d3ee','#c4b5fd'];

  const particles = Array.from({ length: N }, () => {
    const angle = (Math.random() * 360) * Math.PI / 180;
    const speed = Math.random() * 5 + 1.5;
    return {
      x:  cx + (Math.random() - 0.5) * 120,
      y:  cy + (Math.random() - 0.5) * 40,
      vx: Math.cos(angle) * speed * 0.8,
      vy: -(Math.abs(Math.sin(angle)) * speed * 1.8 + Math.random() * 3.5),
      size:    Math.random() * 3.5 + 0.8,
      opacity: 1,
      color:   colors[Math.floor(Math.random() * colors.length)],
    };
  });

  function tick() {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    let alive = false;
    particles.forEach(p => {
      if (p.opacity <= 0.02) return;
      alive = true;
      p.x  += p.vx;
      p.y  += p.vy;
      p.vy += 0.07;         // gravity
      p.vx *= 0.985;        // air friction
      p.opacity -= 0.014;

      ctx.globalAlpha = Math.max(0, p.opacity);
      ctx.fillStyle   = p.color;
      ctx.beginPath();
      ctx.arc(p.x, p.y, p.size * Math.max(0.1, p.opacity), 0, Math.PI * 2);
      ctx.fill();
    });
    ctx.globalAlpha = 1;
    if (alive) requestAnimationFrame(tick);
    else ctx.clearRect(0, 0, canvas.width, canvas.height);
  }
  tick();
}

// ── Typewriter ──────────────────────────────────────────
async function typeText(text, elId = 'c-tagline-text', speed = 55) {
  const el = document.getElementById(elId);
  if (!el) return;
  el.textContent = '';
  for (const ch of text) {
    el.textContent += ch;
    await wait(speed);
  }
}

// ── Notch panel cycling ─────────────────────────────────
const NOTCH_STATES = ['compact', 'music', 'stats', 'clock'];
let   notchCurrent = 'compact';
let   notchTimer   = null;

function switchNotchState(state) {
  if (state === notchCurrent) return;

  const pill = document.getElementById('c-raven-pill');
  const oldEl = document.getElementById(`c-rst-${notchCurrent}`);
  if (oldEl) oldEl.classList.remove('is-active');

  notchCurrent = state;

  // Resize pill
  if (pill) {
    pill.className = 'c-raven-pill';
    if (state !== 'compact') pill.classList.add(`pill-${state}`);
  }

  // Show new state with slight delay (pill expands first)
  setTimeout(() => {
    const newEl = document.getElementById(`c-rst-${state}`);
    if (newEl) newEl.classList.add('is-active');
  }, 280);
}

function startNotchCycling(interval = 3200) {
  let idx = 0;
  notchTimer = setInterval(() => {
    idx = (idx + 1) % NOTCH_STATES.length;
    switchNotchState(NOTCH_STATES[idx]);
  }, interval);
}

// ── Live animated stats (in the notch) ──────────────────
const sV = { cpu: 38, ram: 64 };
const sT = { cpu: 38, ram: 64 };
function animateStats() {
  sT.cpu = Math.max(5,  Math.min(85, sT.cpu + (Math.random() - 0.5) * 14));
  sT.ram = Math.max(30, Math.min(90, sT.ram + (Math.random() - 0.5) * 6));
  sV.cpu = Math.round(sV.cpu + (sT.cpu - sV.cpu) * 0.4);
  sV.ram = Math.round(sV.ram + (sT.ram - sV.ram) * 0.4);

  const cpuF = document.getElementById('c-cpu-fill');
  const ramF = document.getElementById('c-ram-fill');
  const cpuP = document.getElementById('c-cpu-pct');
  const ramP = document.getElementById('c-ram-pct');
  if (cpuF) cpuF.style.width = sV.cpu + '%';
  if (ramF) ramF.style.width = sV.ram + '%';
  if (cpuP) cpuP.textContent = sV.cpu + '%';
  if (ramP) ramP.textContent = sV.ram + '%';
}
setInterval(animateStats, 2400);

// ── MAIN CINEMATIC SEQUENCE ─────────────────────────────
async function runCinematic() {

  // Grab DOM refs
  const rock        = document.getElementById('c-rock');
  const laptopPersp = document.getElementById('c-laptop-persp');
  const lid         = document.getElementById('c-lid');
  const display     = document.getElementById('c-display');
  const bootPhase   = document.getElementById('c-boot');
  const logoWrap    = document.getElementById('c-logo-wrap');
  const desktopPhase= document.getElementById('c-desktop');
  const ravenBar    = document.getElementById('c-raven-bar');
  const taglineWrap = document.getElementById('c-tagline');
  const heroBottom  = document.getElementById('c-hero-bottom');

  // Safety: if key elements missing, bail
  if (!rock || !lid || !bootPhase) return;

  // ── 0.6s: Initial scene visible (CSS handles initial opacity of hero) ──
  await wait(600);

  // ── 0.8s: Rock sinks ──
  rock.classList.add('is-sinking');

  // ── 1.2s: Laptop floats up ──
  await wait(400);
  laptopPersp.classList.add('is-floating');

  // ── 2.6s: Lid opens ──
  await wait(1400);
  lid.classList.add('is-open');

  // ── 3.9s: Screen on + boot logo appears ──
  await wait(1300);
  display.style.background = '#000';
  bootPhase.classList.add('is-visible');
  await wait(150);
  logoWrap.classList.add('is-visible');

  // ── 4.3s: Logo starts glowing ──
  await wait(400);
  logoWrap.classList.add('is-glowing');

  // ── 5.4s: Shatter! Logo explodes into particles ──
  await wait(1100);
  logoWrap.style.opacity = '0';
  logoWrap.style.transform = 'scale(1.3)';
  logoWrap.style.transition = 'opacity 0.2s ease, transform 0.2s ease';
  shatterLogo();

  // ── 5.8s: Hide boot, show desktop ──
  await wait(400);
  bootPhase.classList.remove('is-visible');
  await wait(250);
  desktopPhase.classList.add('is-visible');

  // ── 6.2s: Notch drops in from top of screen ──
  await wait(400);
  ravenBar.classList.add('is-visible');

  // ── 6.8s: Tagline types in ──
  await wait(600);
  if (taglineWrap) taglineWrap.classList.add('is-visible');
  await typeText('Dynamic Notch for Windows');

  // Remove cursor after typing is done
  await wait(800);
  const cursor = document.getElementById('c-cursor');
  if (cursor) cursor.style.display = 'none';

  // ── 8.0s: CTAs appear ──
  if (heroBottom) heroBottom.classList.add('is-visible');

  // ── 8.5s: Start cycling notch panels ──
  await wait(500);
  startNotchCycling(3200);
}

// ── PARALLAX TILT on laptop (mouse move) ────────────────
function initTilt() {
  const scene      = document.getElementById('c-scene');
  const laptopPersp= document.getElementById('c-laptop-persp');
  if (!scene || !laptopPersp) return;

  scene.addEventListener('mousemove', (e) => {
    const rect = scene.getBoundingClientRect();
    const cx = rect.left + rect.width  / 2;
    const cy = rect.top  + rect.height / 2;
    const dx = (e.clientX - cx) / (rect.width  / 2);  // -1 to 1
    const dy = (e.clientY - cy) / (rect.height / 2);  // -1 to 1

    const tiltX = -dy * 6;  // up-down tilt
    const tiltY =  dx * 8;  // left-right tilt

    laptopPersp.style.transform =
      `translateX(-50%) perspective(700px) rotateX(${8 + tiltX}deg) rotateY(${tiltY}deg)`;
  });

  scene.addEventListener('mouseleave', () => {
    laptopPersp.style.transition = 'transform 0.8s cubic-bezier(0.34, 1.56, 0.64, 1.0), bottom 1.6s cubic-bezier(0.34, 1.3, 0.64, 1.0)';
    laptopPersp.style.transform  = 'translateX(-50%) perspective(700px) rotateX(8deg) rotateY(0deg)';
    setTimeout(() => { laptopPersp.style.transition = ''; }, 800);
  });
}

// ── BOOT ────────────────────────────────────────────────
window.addEventListener('DOMContentLoaded', () => {
  // Start animation on load
  runCinematic();
  initTilt();
});
