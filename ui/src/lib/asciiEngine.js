// Canvas ASCII field for the new-chat screen: a flock of symbol bats drifts
// in the background; each click drops a pumpkin that grows outward like a
// ripple, scatters its symbols and fades. Ported from the approved design.
/* eslint-disable */
export class AsciiEngine {
  /** options: { figure: "calabaza" | "fantasma", background: "murcielagos" | "niebla", glyphs: "simbolos" | "letras" } */
  constructor(canvas, options = {}) {
    this.props = options;
    this.canvas = canvas;
    this.drops = [];
    this.cellW = 10;
    this.cellH = 16;
    // The bat layer uses its own finer grid so wings, fingers and ears fit.
    this.batW = 6;
    this.batH = 10;
    // Symbol ramp from light to dense, as in classic ASCII art shading.
    this.ramp = ['.,\'`', ':;-~', '=+<>^', '*?/\\|', '%$&{}', '#@'];
    this.letters = 'abcdefghijklmnopqrstuvwxyz0123456789';
    this.frame = (now) => this.draw(now);
    // Fixed lattice of random values for the background value noise.
    this.lattice = new Float32Array(256 * 256);
    let seed = 1337;
    const rand = () => { seed = (seed * 16807) % 2147483647; return seed / 2147483647; };
    for (let i = 0; i < this.lattice.length; i++) this.lattice[i] = rand();
    // Background flock: smaller bats are farther away, so dimmer and slower.
    this.bats = [];
    for (let k = 0; k < 10; k++) {
      const depth = rand();
      this.bats.push({
        size: 46 + depth * 64,
        depth,
        x: rand(),
        y: 0.08 + rand() * 0.72,
        speed: (18 + depth * 34) * (rand() < 0.5 ? -1 : 1),
        bob: 8 + rand() * 18,
        bobRate: 0.4 + rand() * 0.6,
        flapRate: 5 + rand() * 4,
        phase: rand() * 6.28,
      });
    }
  }

  start() {
    this.reduced = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    this.observer = new ResizeObserver(() => this.layout());
    if (this.canvas) this.observer.observe(this.canvas);
    this.layout();
    this.raf = requestAnimationFrame(this.frame);
  }

  stop() {
    if (this.observer) this.observer.disconnect();
    cancelAnimationFrame(this.raf);
  }

  layout() {
    const canvas = this.canvas;
    if (!canvas) return;
    const ratio = window.devicePixelRatio || 1;
    canvas.width = Math.round(canvas.clientWidth * ratio);
    canvas.height = Math.round(canvas.clientHeight * ratio);
    this.ctx = canvas.getContext('2d');
    this.ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
    this.cols = Math.ceil(canvas.clientWidth / this.cellW);
    this.rows = Math.ceil(canvas.clientHeight / this.cellH);
    this.chars = new Array(this.cols * this.rows).fill(' ');
    this.bcols = Math.ceil(canvas.clientWidth / this.batW);
    this.brows = Math.ceil(canvas.clientHeight / this.batH);
    this.field = new Float32Array(this.bcols * this.brows);
    this.eyes = new Uint8Array(this.bcols * this.brows);
    this.bchars = new Array(this.bcols * this.brows).fill(' ');
    this.blevels = new Int8Array(this.bcols * this.brows).fill(-1);
    this.levels = new Int8Array(this.cols * this.rows).fill(-1);
  }

  // Smooth value noise in [0, 1].
  noise(x, y) {
    const xi = Math.floor(x);
    const yi = Math.floor(y);
    const xf = x - xi;
    const yf = y - yi;
    const at = (i, j) => this.lattice[((j & 255) << 8) | (i & 255)];
    const u = xf * xf * (3 - 2 * xf);
    const v = yf * yf * (3 - 2 * yf);
    const top = at(xi, yi) + (at(xi + 1, yi) - at(xi, yi)) * u;
    const bottom = at(xi, yi + 1) + (at(xi + 1, yi + 1) - at(xi, yi + 1)) * u;
    return top + (bottom - top) * v;
  }

  // Background mist: two octaves stretched along a diagonal and drifting,
  // so grey bands of symbols slowly flow and change shape.
  mist(px, py, t) {
    const a = 0.6;
    const u = (px * Math.cos(a) + py * Math.sin(a)) / 210;
    const v = (-px * Math.sin(a) + py * Math.cos(a)) / 70;
    const warp = this.noise(u * 0.7 + t * 0.05, v * 0.7 - t * 0.03) * 1.6;
    const n = this.noise(u + t * 0.09 + warp, v - t * 0.04) * 0.65
      + this.noise(u * 2.3 - t * 0.06, v * 2.3 + warp) * 0.35;
    return Math.max(0, (n - 0.56) / 0.44);
  }

  // Right-wing skeleton for a flap in [-1, 1] (raised when positive), in
  // units of half the wingspan, y pointing down. The wing rotates from the
  // shoulder: an arm to the wrist, then three fingers fanning out; the
  // membrane between finger tips curves inward like a real bat's.
  wing(flap) {
    const dir = (angle) => [Math.cos(angle), Math.sin(angle)];
    const add = (p, d, length) => [p[0] + d[0] * length, p[1] + d[1] * length];
    const raise = flap * 0.62;
    const fold = 1 - Math.max(0, flap) * 0.16;
    const shoulder = [0.09, -0.07];
    const armAngle = -0.32 - raise;
    const wrist = add(shoulder, dir(armAngle), 0.4 * fold);
    const tips = [
      add(wrist, dir(armAngle + 0.06), 0.58 * fold),
      add(wrist, dir(armAngle + 0.72), 0.52 * fold),
      add(wrist, dir(armAngle + 1.38), 0.43 * fold),
    ];
    const leg = [0.08, 0.24];
    const scallop = (a, b) => {
      const mid = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
      const pull = Math.hypot(b[0] - a[0], b[1] - a[1]) * 0.42;
      const toward = Math.hypot(wrist[0] - mid[0], wrist[1] - mid[1]) || 1;
      const ctrl = [mid[0] + ((wrist[0] - mid[0]) / toward) * pull, mid[1] + ((wrist[1] - mid[1]) / toward) * pull];
      const points = [];
      for (let s = 1; s < 6; s++) {
        const u = s / 6;
        points.push([
          (1 - u) * (1 - u) * a[0] + 2 * (1 - u) * u * ctrl[0] + u * u * b[0],
          (1 - u) * (1 - u) * a[1] + 2 * (1 - u) * u * ctrl[1] + u * u * b[1],
        ]);
      }
      return points;
    };
    const membrane = [shoulder, wrist, tips[0], ...scallop(tips[0], tips[1]), tips[1],
      ...scallop(tips[1], tips[2]), tips[2], ...scallop(tips[2], leg), leg, [0.07, 0.02]];
    const bones = [[shoulder, wrist], [wrist, tips[0]], [wrist, tips[1]], [wrist, tips[2]]];
    return { membrane, bones, wrist, shoulder };
  }

  inside(polygon, x, y) {
    let hit = false;
    for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
      const [xi, yi] = polygon[i];
      const [xj, yj] = polygon[j];
      if ((yi > y) !== (yj > y) && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) hit = !hit;
    }
    return hit;
  }

  segmentDistance(p, a, b) {
    const vx = b[0] - a[0];
    const vy = b[1] - a[1];
    const t = Math.max(0, Math.min(1, ((p[0] - a[0]) * vx + (p[1] - a[1]) * vy) / (vx * vx + vy * vy)));
    return Math.hypot(p[0] - a[0] - vx * t, p[1] - a[1] - vy * t);
  }

  // Bat value at (x, y): 0 empty, otherwise density 0..1; -1 marks an eye.
  // Fur, head and ears are densest; bones are drawn as lines; the membrane
  // is denser near the arm and lighter towards the trailing edge.
  bat(x, y, geo, size, grain) {
    const ax = Math.abs(x);
    const bodyQ = (x / 0.1) ** 2 + ((y - 0.05) / 0.19) ** 2;
    if (bodyQ <= 1) return bodyQ > 0.7 && grain < 0.35 ? 0.62 : 0.95;
    const headQ = (x / 0.085) ** 2 + ((y + 0.17) / 0.075) ** 2;
    if (headQ <= 1) return 1;
    // Ears: tall triangles leaning outwards.
    if (y >= -0.37 && y <= -0.21) {
      const k = (y + 0.37) / 0.16;
      if (ax >= 0.02 + 0.01 * k && ax <= 0.05 + 0.05 * k && ax >= 0.03 + (1 - k) * 0.04) return 0.88;
    }
    // Tail membrane between the legs.
    if (y >= 0.2 && y <= 0.36 && ax <= 0.09 * (1 - (y - 0.2) / 0.16)) return 0.34;
    const p = [ax, y];
    const bone = Math.max(0.02, 3.5 / size);
    for (const [a, b] of geo.bones) if (this.segmentDistance(p, a, b) < bone) return 0.74;
    if (Math.hypot(p[0] - geo.wrist[0], p[1] - geo.wrist[1]) < bone * 1.6) return 0.8;
    if (!this.inside(geo.membrane, ax, y)) return 0;
    const reach = Math.hypot(p[0] - geo.shoulder[0], p[1] - geo.shoulder[1]);
    return Math.max(0.3, 0.52 - reach * 0.16);
  }

  // Bat layer, drawn first on its fine grid with smaller symbols; faint
  // amber eyes on the larger bats.
  drawBats(ctx) {
    ctx.font = '9px "IBM Plex Mono", ui-monospace, monospace';
    const letters = (this.props.glyphs ?? 'simbolos') === 'letras';
    for (let r = 0; r < this.brows; r++) {
      for (let c = 0; c < this.bcols; c++) {
        const i = r * this.bcols + c;
        const v = this.field[i];
        if (!v) { this.blevels[i] = -1; continue; }
        const level = this.eyes[i] ? 0.7 : v * 0.8;
        ctx.fillStyle = this.eyes[i] ? 'rgba(196, 150, 86,' + (0.25 + v * 0.25) + ')'
          : 'rgba(128, 138, 144,' + ((0.05 + v * 0.22) * 1.2) + ')';
        const bucket = letters ? 0 : Math.min(this.ramp.length - 1, Math.floor(level * this.ramp.length));
        if (this.blevels[i] !== bucket || Math.random() < 0.03) {
          this.blevels[i] = bucket;
          const set = letters ? this.letters : this.ramp[bucket];
          this.bchars[i] = set[Math.floor(Math.random() * set.length)];
        }
        ctx.fillText(this.bchars[i], c * this.batW, r * this.batH);
      }
    }
  }

  // Paint the flock into the per-cell background field for this frame.
  paintBats(t, width, height) {
    this.field.fill(0);
    this.eyes.fill(0);
    for (const b of this.bats) {
      const span = width + 2 * b.size;
      const travel = ((b.x * span + b.speed * t) % span + span) % span - b.size;
      const cy = b.y * height + Math.sin(t * b.bobRate + b.phase) * b.bob;
      // Quick downstroke, slower recovery; a slight bank while gliding.
      const p = t * b.flapRate + b.phase;
      const flap = Math.max(-1, Math.min(1, Math.sin(p) + 0.28 * Math.sin(2 * p)));
      const tilt = Math.sin(t * b.bobRate * 0.7 + b.phase) * 0.14;
      const cos = Math.cos(-tilt);
      const sin = Math.sin(-tilt);
      const geo = this.wing(flap);
      const strength = 0.55 + b.depth * 0.45;
      const reach = b.size * 1.05;
      const c0 = Math.max(0, Math.floor((travel - reach) / this.batW));
      const c1 = Math.min(this.bcols - 1, Math.ceil((travel + reach) / this.batW));
      const r0 = Math.max(0, Math.floor((cy - reach * 0.8) / this.batH));
      const r1 = Math.min(this.brows - 1, Math.ceil((cy + reach * 0.6) / this.batH));
      for (let r = r0; r <= r1; r++) {
        for (let c = c0; c <= c1; c++) {
          const dx = c * this.batW + this.batW / 2 - travel;
          const dy = r * this.batH + this.batH / 2 - cy;
          const lx = (dx * cos - dy * sin) / b.size;
          const ly = (dx * sin + dy * cos) / b.size;
          const v = this.bat(lx, ly, geo, b.size, this.hash(c, r, 3));
          if (!v) continue;
          const i = r * this.bcols + c;
          if (v < 0) { this.eyes[i] = 1; this.field[i] = Math.max(this.field[i], strength); continue; }
          if (v * strength > this.field[i]) this.field[i] = v * strength;
        }
      }
    }
  }

  // Pumpkin at (x, y) in units of its radius: returns {kind, shade} or null.
  // Five lobes give it volume; creases between lobes are darker; the carved
  // face glows and lights a thin halo of rind around it.
  pumpkin(x, y) {
    const centers = [-0.62, -0.32, 0, 0.32, 0.62];
    const widths = [0.34, 0.4, 0.42, 0.4, 0.34];
    const heights = [0.56, 0.66, 0.7, 0.66, 0.56];
    let best = Infinity;
    let second = Infinity;
    for (let k = 0; k < 5; k++) {
      const q = ((x - centers[k]) / widths[k]) ** 2 + ((y - 0.12) / heights[k]) ** 2;
      if (q < best) { second = best; best = q; } else if (q < second) { second = q; }
    }
    // Stem: tapered and curving to the right, with a leaf on the left.
    const s = (-0.52 - y) / 0.36;
    if (best > 1) {
      if (s >= 0 && s <= 1 && Math.abs(x - (0.02 + 0.16 * s * s)) <= 0.075 - 0.025 * s) return { kind: 'stem', shade: 0.7 - s * 0.3 };
      const lx = (x + 0.2) * Math.cos(0.5) + (y + 0.6) * Math.sin(0.5);
      const ly = -(x + 0.2) * Math.sin(0.5) + (y + 0.6) * Math.cos(0.5);
      if ((lx / 0.17) ** 2 + (ly / 0.06) ** 2 <= 1) return { kind: 'leaf', shade: 0.55 };
      return null;
    }
    const tri = (cx, top, bottom, half) => y >= top && y <= bottom && Math.abs(x - cx) <= ((y - top) / (bottom - top)) * half;
    const upper = 0.24 + 0.34 * x * x;
    const lower = 0.5 - 0.08 * x * x;
    const toothTop = Math.abs(Math.abs(x) - 0.18) < 0.06 && y < upper + 0.08;
    const toothBottom = Math.abs(x) < 0.06 && y > lower - 0.08;
    const mouth = Math.abs(x) <= 0.52 && y >= upper && y <= lower && !toothTop && !toothBottom;
    const face = tri(-0.25, -0.28, 0.02, 0.17) || tri(0.25, -0.28, 0.02, 0.17) || tri(0, 0.04, 0.17, 0.07) || mouth;
    if (face) return { kind: 'face', shade: 1 };
    const nearFace = tri(-0.25, -0.33, 0.06, 0.24) || tri(0.25, -0.33, 0.06, 0.24)
      || (Math.abs(x) <= 0.58 && y >= upper - 0.05 && y <= lower + 0.05);
    const crease = second - best < 0.18;
    const rim = best > 0.78;
    return { kind: crease ? 'crease' : nearFace ? 'halo' : 'body', shade: rim ? 0.45 : 1 - best * 0.5 };
  }

  ghost(x, y) {
    const wave = 0.62 + 0.07 * Math.sin(x * 16);
    const body = Math.abs(x) <= 0.55 && y <= wave && (y >= -0.2 || (x * x) / 0.3 + ((y + 0.2) * (y + 0.2)) / 0.42 <= 1);
    if (!body) return null;
    const eye = (cx) => ((x - cx) * (x - cx)) / 0.008 + ((y + 0.1) * (y + 0.1)) / 0.02 <= 1;
    const mouth = (x * x) / 0.012 + ((y - 0.16) * (y - 0.16)) / 0.01 <= 1;
    return eye(-0.19) || eye(0.19) || mouth ? { kind: 'void', shade: 0 } : { kind: 'ghost', shade: 1 - Math.abs(x) };
  }

  // Stable pseudo-random value in [0, 1) per cell and drop.
  hash(a, b, c) {
    const h = Math.sin(a * 127.1 + b * 311.7 + c * 74.7) * 43758.5453;
    return h - Math.floor(h);
  }

  glyph(i, level) {
    const letters = (this.props.glyphs ?? 'simbolos') === 'letras';
    const bucket = letters ? 0 : Math.min(this.ramp.length - 1, Math.floor(level * this.ramp.length));
    if (this.levels[i] !== bucket || Math.random() < 0.035) {
      this.levels[i] = bucket;
      const set = letters ? this.letters : this.ramp[bucket];
      this.chars[i] = set[Math.floor(Math.random() * set.length)];
    }
    return this.chars[i];
  }

  draw(now) {
    this.raf = requestAnimationFrame(this.frame);
    const ctx = this.ctx;
    if (!ctx || document.hidden) return;
    // The background moves at ~30 fps; drops animate every frame.
    if (!this.drops.length && now - (this.lastMist || 0) < 33) return;
    this.lastMist = now;
    const width = this.canvas.clientWidth;
    const height = this.canvas.clientHeight;
    const t = this.reduced ? 0 : now / 1000;
    const isGhost = (this.props.figure ?? 'calabaza') === 'fantasma';
    ctx.clearRect(0, 0, width, height);
    ctx.font = '13px "IBM Plex Mono", ui-monospace, monospace';
    ctx.textBaseline = 'top';
    this.drops = this.drops.filter((drop) => now - drop.t0 < 3400);
    const live = this.drops.map((drop) => ({ ...drop, age: (now - drop.t0) / 1000 }));
    const bats = (this.props.background ?? 'murcielagos') === 'murcielagos';
    if (bats) {
      this.paintBats(t, width, height);
      this.drawBats(ctx);
      ctx.font = '13px "IBM Plex Mono", ui-monospace, monospace';
    }
    for (let r = 0; r < this.rows; r++) {
      for (let c = 0; c < this.cols; c++) {
        const i = r * this.cols + c;
        const px = c * this.cellW + this.cellW / 2;
        const py = r * this.cellH + this.cellH / 2;
        let figure = null;
        let figureAlpha = 0;
        for (const drop of live) {
          // The figure itself is the wave: it lands small, then it and two
          // trailing echoes grow outward, scatter their symbols and fade.
          const echoes = this.reduced ? 1 : 3;
          for (let k = 0; k < echoes; k++) {
            const age = drop.age - k * 0.2;
            if (age < 0) continue;
            const size = this.reduced ? 100 : 34 + age * 330 + age * age * 40;
            const strength = (this.reduced ? Math.max(0, 1 - age / 1.6) : Math.exp(-age * 1.15)) * (1 - k * 0.32);
            if (strength < 0.03) continue;
            // Scatter: symbols drift a little off the shape and drop out as it spreads.
            const spread = age * 9;
            const jx = (this.hash(c, r, drop.id + k) - 0.5) * spread;
            const jy = (this.hash(r, c, drop.id + k + 7) - 0.5) * spread;
            if (this.hash(c + 31, r + 17, drop.id + k) < Math.min(0.85, age * 0.42)) continue;
            const hx = (px + jx - drop.x) / size;
            const hy = (py + jy - drop.y) / size;
            const hit = isGhost ? this.ghost(hx, hy) : this.pumpkin(hx, hy);
            const pop = Math.min(1, drop.age / 0.12);
            const alpha = strength * pop;
            if (hit && alpha > figureAlpha) { figure = hit; figureAlpha = alpha; }
          }
        }
        let level;
        if (figure && figure.kind !== 'void') {
          // Muted palette at reduced opacity, so the figure stays in the background.
          const a = figureAlpha * 0.62;
          const sh = figure.shade;
          switch (figure.kind) {
            case 'face': level = 0.99; ctx.fillStyle = 'rgba(214, 186, 122,' + a + ')'; break;
            case 'halo': level = 0.8; ctx.fillStyle = 'rgba(196, 140, 84,' + a + ')'; break;
            case 'crease': level = 0.42; ctx.fillStyle = 'rgba(98, 56, 30,' + a + ')'; break;
            case 'stem': level = 0.7; ctx.fillStyle = 'rgba(' + Math.round(84 + 26 * sh) + ', ' + Math.round(100 + 26 * sh) + ', 66,' + a + ')'; break;
            case 'leaf': level = 0.55; ctx.fillStyle = 'rgba(92, 118, 70,' + a + ')'; break;
            case 'ghost': level = 0.5 + sh * 0.45; ctx.fillStyle = 'rgba(196, 204, 208,' + (a * (0.55 + sh * 0.45)) + ')'; break;
            default: {
              // Body: lighter at each lobe's centre, darker towards the rim.
              level = 0.45 + sh * 0.4;
              ctx.fillStyle = 'rgba(' + Math.round(128 + 62 * sh) + ', ' + Math.round(70 + 44 * sh) + ', ' + Math.round(32 + 22 * sh) + ',' + a + ')';
            }
          }
        } else if (figure && figure.kind === 'void') {
          level = 0;
          ctx.fillStyle = 'rgba(0, 0, 0, 0)';
        } else {
          const m = bats ? 0 : this.mist(px, py, t);
          if (m > 0) {
            level = m * 0.75;
            ctx.fillStyle = 'rgba(128, 138, 144,' + ((0.05 + m * 0.2) * 1.2) + ')';
          } else {
            this.levels[i] = -1;
            continue;
          }
        }
        ctx.fillText(this.glyph(i, level), c * this.cellW, r * this.cellH);
      }
    }
  }

  /** Drop a figure at canvas-relative coordinates. */
  splash(x, y) {
    this.drops.push({ x, y, t0: performance.now(), id: Math.random() * 1000 });
    if (this.drops.length > 6) this.drops.shift();
  }
}
