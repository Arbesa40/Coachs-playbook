// Local files and YouTube share one interface: play, pause, seek, time, duration, setRate, rates, step, destroy.

export class LocalSource {
  kind = "local";
  frameExact = true;
  constructor(el, fps = 25) {
    this.el = el;
    this.fps = fps > 1 ? fps : 25;
  }
  play() {
    return this.el.play().catch(() => {});
  }
  pause() {
    this.el.pause();
  }
  get paused() {
    return this.el.paused;
  }
  seek(t) {
    this.el.currentTime = Math.max(0, Math.min(t, this.duration() || t));
  }
  time() {
    return this.el.currentTime || 0;
  }
  duration() {
    return Number.isFinite(this.el.duration) ? this.el.duration : 0;
  }
  setRate(r) {
    this.el.playbackRate = r;
  }
  rates() {
    return [0.1, 0.25, 0.5, 0.75, 1, 1.25, 1.5, 2];
  }
  step(dir) {
    this.pause();
    this.seek(this.time() + dir / this.fps);
  }
  destroy() {}
}

let ytApi;
function loadYouTubeApi() {
  if (ytApi) return ytApi;
  ytApi = new Promise((resolve, reject) => {
    if (window.YT?.Player) return resolve(window.YT);
    const prev = window.onYouTubeIframeAPIReady;
    window.onYouTubeIframeAPIReady = () => {
      prev?.();
      resolve(window.YT);
    };
    const s = document.createElement("script");
    s.src = "https://www.youtube.com/iframe_api";
    s.onerror = () => {
      ytApi = null;
      reject(new Error("YouTube needs an internet connection."));
    };
    document.head.appendChild(s);
  });
  return ytApi;
}

export class YouTubeSource {
  kind = "youtube";
  frameExact = false; // YouTube only allows approximate stepping
  onError = null;

  static async create(container, videoId) {
    const YT = await loadYouTubeApi();
    const src = new YouTubeSource();
    await new Promise((resolve) => {
      src.player = new YT.Player(container, {
        videoId,
        width: "100%",
        height: "100%",
        playerVars: { rel: 0, playsinline: 1, controls: 0, disablekb: 1, modestbranding: 1, origin: location.origin },
        events: {
          onReady: () => resolve(),
          onError: (e) => src.onError?.(e.data),
        },
      });
    });
    return src;
  }
  play() {
    this.player.playVideo();
  }
  pause() {
    this.player.pauseVideo();
  }
  get paused() {
    const s = this.player.getPlayerState?.();
    return s !== 1 && s !== 3; // playing or buffering
  }
  seek(t) {
    this.player.seekTo(Math.max(0, t), true);
  }
  time() {
    return this.player.getCurrentTime?.() || 0;
  }
  duration() {
    return this.player.getDuration?.() || 0;
  }
  setRate(r) {
    this.player.setPlaybackRate(r);
  }
  rates() {
    return this.player.getAvailablePlaybackRates?.() || [0.25, 0.5, 1, 1.5, 2];
  }
  step(dir) {
    this.pause();
    this.seek(this.time() + dir / 30);
  }
  destroy() {
    try {
      this.player.destroy();
    } catch {}
  }
}

export function youtubeLink(id, t = 0) {
  return `https://youtu.be/${id}?t=${Math.floor(t)}`;
}
