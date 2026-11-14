// Landscape full screen via MainActivity's CoachScreen on Android, the web Fullscreen API elsewhere.
export function setImmersiveLandscape(on) {
  try {
    if (window.CoachScreen?.immersiveLandscape) {
      window.CoachScreen.immersiveLandscape(Boolean(on));
      return;
    }
  } catch {}
  try {
    if (on) {
      const p = document.fullscreenElement ? Promise.resolve() : document.documentElement.requestFullscreen?.();
      Promise.resolve(p)
        .then(() => screen.orientation?.lock?.("landscape"))
        .catch(() => {});
    } else {
      screen.orientation?.unlock?.();
      if (document.fullscreenElement) document.exitFullscreen?.().catch(() => {});
    }
  } catch {}
}
