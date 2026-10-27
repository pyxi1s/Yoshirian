// Loading cursor data from the backend.
import { invoke } from "../../core/tauri.js";
import { S } from "../../core/state.js";
import { toast } from "../../core/feedback.js";
import { render } from "../../core/render.js";

export async function loadCursor() {
  try { S.cur = await invoke("cursor_info"); } catch (er) { S.cur = { error: String(er) }; }
  render();
}
export async function cursorCall(cmd, args) {
  try { await invoke(cmd, args); S.cv++; } catch (er) { toast(er); }
  await loadCursor();
}

/** Call when cursors were added from outside the Cursor tab (e.g. a downloaded cursor pack). */
export function cursorsChanged() {
  S.cv++; S.cur = null;
  if (S.tab === "cursor") loadCursor();
}
