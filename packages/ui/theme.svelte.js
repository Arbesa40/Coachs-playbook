// The resolved theme is written to <html data-theme>; styles.css does the rest.

const KEY = "theme";
const media = typeof matchMedia === "function" ? matchMedia("(prefers-color-scheme: light)") : null;

function readPref() {
  try {
    const v = localStorage.getItem(KEY);
    return v === "light" || v === "dark" ? v : "system";
  } catch {
    return "system";
  }
}

export const theme = $state({ pref: readPref(), resolved: "dark" });

function apply() {
  theme.resolved = theme.pref === "system" ? (media?.matches ? "light" : "dark") : theme.pref;
  const root = document.documentElement;
  root.dataset.theme = theme.resolved;
  // Some WebView builds don't repaint the page canvas when only a variable changes.
  root.style.background = getComputedStyle(root).getPropertyValue("--bg").trim();
}

export function setTheme(pref) {
  theme.pref = pref === "light" || pref === "dark" ? pref : "system";
  try {
    localStorage.setItem(KEY, theme.pref);
  } catch {}
  apply();
}

export function initTheme() {
  apply();
  media?.addEventListener?.("change", () => theme.pref === "system" && apply());
}

export const THEME_OPTIONS = [
  ["system", "Match the system"],
  ["light", "Light"],
  ["dark", "Dark"],
];
