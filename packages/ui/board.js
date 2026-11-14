// board = { v: 2, items: [{id,type,x,y}], lines: [{id,kind,x1,y1,x2,y2,step}], steps: [{dur,pos}] }, coordinates 0..1.

export const uid = () => Math.random().toString(36).slice(2, 10);

export function emptyBoard() {
  return { v: 2, items: [], lines: [], steps: [] };
}

// Accept old drill diagrams (an array of shapes) and anything partial.
export function normalize(b) {
  if (!b) return emptyBoard();
  if (Array.isArray(b)) {
    const out = emptyBoard();
    for (const it of b) {
      if (["arrow", "pass", "dribble"].includes(it.type)) {
        out.lines.push({ id: uid(), kind: it.type === "arrow" ? "run" : it.type, x1: it.x, y1: it.y, x2: it.x2, y2: it.y2, step: 0 });
      } else {
        out.items.push({ id: uid(), type: it.type, x: it.x, y: it.y, label: it.label ?? "" });
      }
    }
    return out;
  }
  return {
    v: 2,
    items: (b.items ?? []).map((i) => ({ ...i, id: i.id ?? uid() })),
    lines: (b.lines ?? []).map((l) => ({ step: 0, ...l, id: l.id ?? uid() })),
    steps: (b.steps ?? []).map((s) => ({ dur: s.dur ?? 1.5, pos: s.pos ?? {} })),
  };
}

export const frameCount = (b) => 1 + (b?.steps?.length ?? 0);

// Positions of every item at a whole step (0 = starting positions).
export function positionsAt(b, frame) {
  const pos = {};
  for (const it of b.items) pos[it.id] = { x: it.x, y: it.y };
  for (let i = 0; i < Math.min(frame, b.steps.length); i++) {
    for (const [id, p] of Object.entries(b.steps[i].pos)) if (pos[id]) pos[id] = { ...p };
  }
  return pos;
}

const ease = (t) => (t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2);

// Positions at a fractional step (for playback).
export function positionsAtT(b, t) {
  const f = Math.floor(t);
  const a = positionsAt(b, f);
  if (t <= f || f >= b.steps.length) return a;
  const bpos = positionsAt(b, f + 1);
  const k = ease(t - f);
  const out = {};
  for (const id of Object.keys(a)) {
    const p = a[id], q = bpos[id] ?? p;
    out[id] = { x: p.x + (q.x - p.x) * k, y: p.y + (q.y - p.y) * k };
  }
  return out;
}

// Total playback time (seconds) and the time at which each step starts.
export function timeline(b) {
  const starts = [0];
  let total = 0;
  for (const s of b.steps) {
    total += s.dur ?? 1.5;
    starts.push(total);
  }
  return { total, starts };
}

// Convert seconds of playback into a fractional step.
export function secondsToT(b, sec) {
  const { starts } = timeline(b);
  for (let i = 0; i < b.steps.length; i++) {
    if (sec < starts[i + 1]) return i + (sec - starts[i]) / (b.steps[i].dur || 1.5);
  }
  return b.steps.length;
}

// Simple undo/redo over JSON snapshots.
export function createHistory(limit = 120) {
  let past = [];
  let future = [];
  return {
    push(board) {
      past.push(JSON.stringify(board));
      if (past.length > limit) past.shift();
      future = [];
    },
    undo(current) {
      if (!past.length) return null;
      future.push(JSON.stringify(current));
      return JSON.parse(past.pop());
    },
    redo(current) {
      if (!future.length) return null;
      past.push(JSON.stringify(current));
      return JSON.parse(future.pop());
    },
    get canUndo() {
      return past.length > 0;
    },
    get canRedo() {
      return future.length > 0;
    },
    clear() {
      past = [];
      future = [];
    },
  };
}

export const DEFAULT_STYLE = {
  home: "#2563eb",
  away: "#dc2626",
  shape: "circle", // circle | shirt | triangle | square
  size: 1, // 0.7 .. 1.6
  numbers: true,
  equipColor: "#f97316",
  lineColor: "#ffffff",
  lineWidth: 1,
};
