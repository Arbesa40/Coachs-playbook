// surface(sport) -> { w, h, pad, bg, line, svg, k }: field size in metres and its markings as SVG.

const f = (n) => Math.round(n * 100) / 100;
const rect = (x, y, w, h, extra = "") => `<rect x="${f(x)}" y="${f(y)}" width="${f(w)}" height="${f(h)}" ${extra}/>`;
const line = (x1, y1, x2, y2, extra = "") => `<line x1="${f(x1)}" y1="${f(y1)}" x2="${f(x2)}" y2="${f(y2)}" ${extra}/>`;
const circle = (cx, cy, r, extra = "") => `<circle cx="${f(cx)}" cy="${f(cy)}" r="${f(r)}" ${extra}/>`;
const dot = (cx, cy, r) => `<circle cx="${f(cx)}" cy="${f(cy)}" r="${f(r)}" fill="currentColor" stroke="none"/>`;
// Arc from angle a0 to a1 (degrees, 0 = +x, clockwise because SVG y points down).
function arc(cx, cy, r, a0, a1, extra = "") {
  const p = (a) => [cx + r * Math.cos((a * Math.PI) / 180), cy + r * Math.sin((a * Math.PI) / 180)];
  const [x0, y0] = p(a0);
  const [x1, y1] = p(a1);
  const large = Math.abs(a1 - a0) > 180 ? 1 : 0;
  const sweep = a1 > a0 ? 1 : 0;
  return `<path d="M${f(x0)} ${f(y0)} A${f(r)} ${f(r)} 0 ${large} ${sweep} ${f(x1)} ${f(y1)}" ${extra}/>`;
}
const dashed = 'stroke-dasharray="1 1"';

function football() {
  const w = 105, h = 68, cy = h / 2;
  const s = [rect(0, 0, w, h), line(w / 2, 0, w / 2, h), circle(w / 2, cy, 9.15), dot(w / 2, cy, 0.3)];
  for (const [x, dir] of [[0, 1], [w, -1]]) {
    s.push(rect(dir > 0 ? 0 : w - 16.5, cy - 20.16, 16.5, 40.32));
    s.push(rect(dir > 0 ? 0 : w - 5.5, cy - 9.16, 5.5, 18.32));
    s.push(rect(dir > 0 ? -2 : w, cy - 3.66, 2, 7.32));
    s.push(dot(x + dir * 11, cy, 0.3));
    // penalty arc: the part of the 9.15 m circle outside the box
    const a = (Math.acos(5.5 / 9.15) * 180) / Math.PI;
    s.push(dir > 0 ? arc(11, cy, 9.15, -a, a) : arc(w - 11, cy, 9.15, 180 - a, 180 + a));
    for (const yy of [0, h]) s.push(arc(x, yy, 1, yy === 0 ? (dir > 0 ? 0 : 90) : (dir > 0 ? -90 : 180), yy === 0 ? (dir > 0 ? 90 : 180) : (dir > 0 ? 0 : 270)));
  }
  return { w, h, pad: 3, bg: "#1f5f32", line: "#e8efe9", k: 1, svg: s.join("") };
}

function basketball() {
  const w = 28, h = 15, cy = h / 2;
  const s = [rect(0, 0, w, h), line(w / 2, 0, w / 2, h), circle(w / 2, cy, 1.8)];
  for (const dir of [1, -1]) {
    const bx = dir > 0 ? 1.575 : w - 1.575; // basket centre
    const base = dir > 0 ? 0 : w;
    // key (paint) 5.8 m long, 4.9 m wide
    s.push(rect(dir > 0 ? 0 : w - 5.8, cy - 2.45, 5.8, 4.9));
    s.push(circle(base + dir * 5.8, cy, 1.8));
    // three-point line: straight 0.9 m from sidelines, arc r 6.75 from the basket
    const yEdge = 0.9;
    const dy = cy - yEdge;
    const dx = Math.sqrt(6.75 ** 2 - dy ** 2);
    s.push(line(base, yEdge, bx + dir * dx, yEdge));
    s.push(line(base, h - yEdge, bx + dir * dx, h - yEdge));
    const a = (Math.asin(dy / 6.75) * 180) / Math.PI;
    s.push(dir > 0 ? arc(bx, cy, 6.75, -a, a) : arc(bx, cy, 6.75, 180 - a, 180 + a));
    // no-charge semicircle, backboard, ring
    s.push(dir > 0 ? arc(bx, cy, 1.25, -90, 90) : arc(bx, cy, 1.25, 90, 270));
    s.push(line(base + dir * 1.2, cy - 0.9, base + dir * 1.2, cy + 0.9, 'stroke-width=".12"'));
    s.push(circle(bx, cy, 0.23));
  }
  return { w, h, pad: 1.2, bg: "#b9874f", line: "#fdf6ec", k: 0.32, svg: s.join("") };
}

function rugby() {
  // 100 m between try lines + 10 m in-goal each end, 70 m wide.
  const w = 120, h = 70;
  const tl = 10, tr = 110, half = 60;
  const s = [rect(0, 0, w, h), line(tl, 0, tl, h, 'stroke-width=".5"'), line(tr, 0, tr, h, 'stroke-width=".5"'), line(half, 0, half, h)];
  s.push(line(tl + 22, 0, tl + 22, h), line(tr - 22, 0, tr - 22, h));
  s.push(line(half - 10, 0, half - 10, h, dashed), line(half + 10, 0, half + 10, h, dashed));
  s.push(line(tl + 5, 0, tl + 5, h, dashed), line(tr - 5, 0, tr - 5, h, dashed));
  // 5 m and 15 m lines in from touch
  for (const y of [5, 15, h - 15, h - 5]) s.push(line(tl, y, tr, y, 'stroke-dasharray="1 4"'));
  // posts (5.6 m apart) as H on each try line
  for (const x of [tl, tr]) s.push(line(x, h / 2 - 2.8, x, h / 2 + 2.8, 'stroke-width=".8"'));
  return { w, h, pad: 3, bg: "#2a6b37", line: "#eef3ee", k: 1, svg: s.join("") };
}

function hockey() {
  const w = 91.4, h = 55, cy = h / 2;
  const s = [rect(0, 0, w, h), line(w / 2, 0, w / 2, h), line(22.9, 0, 22.9, h), line(w - 22.9, 0, w - 22.9, h)];
  for (const dir of [1, -1]) {
    const base = dir > 0 ? 0 : w;
    const g = 3.66 / 2; // goal half-width
    // shooting circle: quarter arcs r 14.63 from each post + straight between
    const r = 14.63;
    s.push(dir > 0 ? arc(base, cy - g, r, -90, 0) : arc(base, cy - g, r, 180, 270));
    s.push(line(base + dir * r, cy - g, base + dir * r, cy + g));
    s.push(dir > 0 ? arc(base, cy + g, r, 0, 90) : arc(base, cy + g, r, 90, 180));
    // dashed 5 m outside the circle
    s.push(dir > 0 ? arc(base, cy - g, r + 5, -90, 0, dashed) : arc(base, cy - g, r + 5, 180, 270, dashed));
    s.push(line(base + dir * (r + 5), cy - g, base + dir * (r + 5), cy + g, dashed));
    s.push(dir > 0 ? arc(base, cy + g, r + 5, 0, 90, dashed) : arc(base, cy + g, r + 5, 90, 180, dashed));
    s.push(dot(base + dir * 6.4, cy, 0.25));
    s.push(rect(dir > 0 ? -1.2 : w, cy - g, 1.2, 3.66));
  }
  return { w, h, pad: 2.5, bg: "#1d5fa8", line: "#f2f6fb", k: 0.9, svg: s.join("") };
}

function volleyball() {
  const w = 18, h = 9;
  const s = [rect(0, 0, w, h), line(6, 0, 6, h), line(12, 0, 12, h)];
  s.push(line(w / 2, -0.6, w / 2, h + 0.6, 'stroke-width=".18"')); // net
  for (const x of [w / 2 - 0.05, w / 2 + 0.05]) s.push(dot(x, -0.6, 0.12), dot(x, h + 0.6, 0.12));
  return { w, h, pad: 1.5, bg: "#d0743a", line: "#fff7ef", k: 0.2, svg: s.join("") };
}

function netball() {
  const w = 30.5, h = 15.25, cy = h / 2;
  const s = [rect(0, 0, w, h), line(w / 3, 0, w / 3, h), line((2 * w) / 3, 0, (2 * w) / 3, h), circle(w / 2, cy, 0.45)];
  s.push(arc(0, cy, 4.9, -90, 90), arc(w, cy, 4.9, 90, 270));
  s.push(dot(0.15, cy, 0.19), dot(w - 0.15, cy, 0.19));
  return { w, h, pad: 1.2, bg: "#2d6c8f", line: "#f3f8fb", k: 0.3, svg: s.join("") };
}

const SURFACES = { football, basketball, rugby, hockey, volleyball, netball };
const cache = {};

export const SPORTS = Object.keys(SURFACES);

export function surface(sport = "football") {
  const key = SURFACES[sport] ? sport : "football";
  return (cache[key] ??= SURFACES[key]());
}
