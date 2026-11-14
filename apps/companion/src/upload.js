import { invoke } from "@tauri-apps/api/core";

const CHUNK = 2 * 1024 * 1024;

// Copy a Blob/File into app storage in small pieces (safe on 2 GB phones) and queue it for the PC.
export async function saveForPc(blob, name, kind, meta, onProgress) {
  const id = await invoke("media_begin", { name });
  for (let off = 0; off < blob.size; off += CHUNK) {
    const part = new Uint8Array(await blob.slice(off, off + CHUNK).arrayBuffer());
    await invoke("media_chunk", part, { headers: { "x-file-id": id } });
    onProgress?.(Math.min(1, (off + CHUNK) / blob.size));
  }
  return invoke("media_finish", { id, kind, meta });
}
