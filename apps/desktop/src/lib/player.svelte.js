// Reactive playback state around whichever Source is active.
export function createPlayer() {
  const p = $state({
    source: null,
    video: null, // the video record being played
    time: 0,
    duration: 0,
    paused: true,
    rate: 1,
    loop: null, // { start, end }
    onTick: null, // called each frame with the time (freeze-frames, playlists)
  });
  $effect.root(() => {
    let raf;
    let last = -1;
    const tick = () => {
      const s = p.source;
      if (s) {
        const t = s.time();
        if (Math.abs(t - last) > 0.03) {
          last = t;
          p.time = t;
        }
        const d = s.duration();
        if (d && d !== p.duration) p.duration = d;
        const paused = s.paused;
        if (paused !== p.paused) p.paused = paused;
        if (p.loop && !paused && t >= p.loop.end) s.seek(p.loop.start);
        p.onTick?.(t);
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });
  return p;
}

export function togglePlay(p) {
  if (!p.source) return;
  p.source.paused ? p.source.play() : p.source.pause();
}

export function seek(p, t) {
  if (!p.source) return;
  p.source.seek(t);
  p.time = t;
}
