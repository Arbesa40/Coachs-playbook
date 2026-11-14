// Shapes use normalized 0..1 coordinates so they fit any player size, local or YouTube, and exports.

export const TOOLS = [
  { id: "arrow", label: "Arrow", key: "a" },
  { id: "line", label: "Line", key: "l" },
  { id: "circle", label: "Circle", key: "c" },
  { id: "free", label: "Freehand", key: "f" },
  { id: "text", label: "Text", key: "x" },
  { id: "spotlight", label: "Spotlight", key: "s" },
  { id: "angle", label: "Angle", key: "n" },
];

export function angleDeg(a, b, c, w = 1, h = 1) {
  const v1 = [(a[0] - b[0]) * w, (a[1] - b[1]) * h];
  const v2 = [(c[0] - b[0]) * w, (c[1] - b[1]) * h];
  const d = Math.atan2(v2[1], v2[0]) - Math.atan2(v1[1], v1[0]);
  let deg = Math.abs((d * 180) / Math.PI);
  if (deg > 180) deg = 360 - deg;
  return Math.round(deg);
}

export function render(ctx, shapes, w, h) {
  const scale = Math.max(1, w / 900);
  const P = ([x, y]) => [x * w, y * h];
  for (const s of shapes) {
    ctx.save();
    ctx.strokeStyle = ctx.fillStyle = s.color || "#facc15";
    ctx.lineWidth = 3 * scale;
    ctx.lineCap = ctx.lineJoin = "round";
    const pts = s.pts.map(P);
    if (s.type === "spotlight" && pts.length >= 2) {
      const r = Math.hypot(pts[1][0] - pts[0][0], pts[1][1] - pts[0][1]);
      ctx.fillStyle = "rgba(0,0,0,0.55)";
      ctx.beginPath();
      ctx.rect(0, 0, w, h);
      ctx.arc(pts[0][0], pts[0][1], r, 0, Math.PI * 2, true);
      ctx.fill("evenodd");
      ctx.strokeStyle = "#fff";
      ctx.beginPath();
      ctx.arc(pts[0][0], pts[0][1], r, 0, Math.PI * 2);
      ctx.stroke();
    } else if (s.type === "circle" && pts.length >= 2) {
      const rx = Math.abs(pts[1][0] - pts[0][0]);
      const ry = Math.max(4, rx * 0.35);
      ctx.beginPath();
      ctx.ellipse(pts[0][0], pts[0][1], rx, ry, 0, 0, Math.PI * 2);
      ctx.stroke();
    } else if ((s.type === "line" || s.type === "arrow") && pts.length >= 2) {
      const [a, b] = [pts[0], pts[pts.length - 1]];
      ctx.beginPath();
      ctx.moveTo(...a);
      ctx.lineTo(...b);
      ctx.stroke();
      if (s.type === "arrow") {
        const ang = Math.atan2(b[1] - a[1], b[0] - a[0]);
        const L = 16 * scale;
        ctx.beginPath();
        ctx.moveTo(...b);
        ctx.lineTo(b[0] - L * Math.cos(ang - 0.45), b[1] - L * Math.sin(ang - 0.45));
        ctx.lineTo(b[0] - L * Math.cos(ang + 0.45), b[1] - L * Math.sin(ang + 0.45));
        ctx.closePath();
        ctx.fill();
      }
    } else if (s.type === "free" && pts.length > 1) {
      ctx.beginPath();
      ctx.moveTo(...pts[0]);
      for (const p of pts.slice(1)) ctx.lineTo(...p);
      ctx.stroke();
    } else if (s.type === "text" && pts.length) {
      ctx.font = `600 ${Math.round(20 * scale)}px Segoe UI, sans-serif`;
      ctx.lineWidth = 4 * scale;
      ctx.strokeStyle = "rgba(0,0,0,.8)";
      ctx.strokeText(s.text || "", ...pts[0]);
      ctx.fillText(s.text || "", ...pts[0]);
    } else if (s.type === "angle" && pts.length >= 2) {
      ctx.beginPath();
      ctx.moveTo(...pts[0]);
      ctx.lineTo(...pts[1]);
      if (pts[2]) ctx.lineTo(...pts[2]);
      ctx.stroke();
      if (pts.length === 3) {
        const deg = angleDeg(s.pts[0], s.pts[1], s.pts[2], w, h);
        ctx.font = `600 ${Math.round(18 * scale)}px Segoe UI, sans-serif`;
        ctx.fillText(deg + "°", pts[1][0] + 8 * scale, pts[1][1] - 8 * scale);
      }
    }
    ctx.restore();
  }
}

// PNG data URL of the drawings at a given pixel size (for burn-in export).
export function overlayPng(shapes, w, h) {
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  render(c.getContext("2d"), shapes, w, h);
  return c.toDataURL("image/png");
}

// Snapshot of the current frame (local video only) with drawings on top.
export function snapshotPng(videoEl, shapes, w = 960, h = 540) {
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  const ctx = c.getContext("2d");
  ctx.fillStyle = "#111";
  ctx.fillRect(0, 0, w, h);
  if (videoEl) {
    try {
      ctx.drawImage(videoEl, 0, 0, w, h);
    } catch {}
  }
  render(ctx, shapes, w, h);
  try {
    return c.toDataURL("image/png");
  } catch {
    // Frame could not be captured; keep the drawings on a plain background.
    const c2 = document.createElement("canvas");
    c2.width = w;
    c2.height = h;
    const x = c2.getContext("2d");
    x.fillStyle = "#111";
    x.fillRect(0, 0, w, h);
    render(x, shapes, w, h);
    return c2.toDataURL("image/png");
  }
}
