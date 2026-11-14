// Translations come from /locales; add a language by copying en.json.
import en from "../../locales/en.json";

const languages = { en };
let current = "en";
try {
  current = localStorage.getItem("lang") || "en";
} catch {}

export const availableLanguages = Object.keys(languages);

export function setLanguage(code) {
  if (!languages[code]) return;
  current = code;
  try {
    localStorage.setItem("lang", code);
  } catch {}
}

// Look up a key like "nav.analysis"; falls back to English, then to the key itself.
export function t(key) {
  const get = (dict) => key.split(".").reduce((o, k) => (o == null ? o : o[k]), dict);
  return get(languages[current]) ?? get(en) ?? key;
}
