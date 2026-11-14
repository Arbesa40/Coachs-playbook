// In-app dialogs; ask() falls back to window.confirm when no <DialogHost /> is mounted.

export const dialogs = $state({ current: null, hosted: false });

export function confirmDialog(message, { ok = "OK", cancel = "Cancel", danger = null } = {}) {
  return new Promise((resolve) => {
    const isDanger = danger ?? /\b(delete|remove|discard|restor)/i.test(message);
    const [title, ...rest] = String(message).split("\n\n");
    dialogs.current = { title, body: rest.join("\n\n"), ok: ok === "OK" ? firstVerb(title) : ok, cancel, danger: isDanger, resolve };
  });
}

function firstVerb(title) {
  const m = /^(Delete|Remove|Discard|Restore|Record|Start|Open)\b/i.exec(title.trim());
  return m ? m[1][0].toUpperCase() + m[1].slice(1).toLowerCase() : "OK";
}

// Asks for a line of text. Resolves to the text, or null when cancelled.
export function textDialog(title, value = "", { ok = "OK", cancel = "Cancel", placeholder = "", maxlength = 200 } = {}) {
  return new Promise((resolve) => {
    dialogs.current = { title, body: "", ok, cancel, danger: false, input: { value: value ?? "", placeholder, maxlength }, resolve };
  });
}

export function closeDialog(result) {
  const d = dialogs.current;
  dialogs.current = null;
  d?.resolve(result);
}
