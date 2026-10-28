// Asking the backend whether a newer app release exists, and what to do with the answer.
import { invoke } from "../../core/tauri.js";
import { S } from "../../core/state.js";
import { toast, call } from "../../core/feedback.js";
import { render, renderChrome } from "../../core/render.js";
import { openUpdatePopup } from "./popup.js";

/** An update the user should be told about: exists, and was not skipped. */
export const isPending = () => !!(S.appUpdate && S.appUpdate.latest && !S.appUpdate.skipped);

let announced = false; // the popup opens by itself once per program start

function redraw() {
  renderChrome();                          // title bar "Update" button
  if (S.tab === "settings") render();      // status card in Settings -> Updates
}

/** `auto` = the check at program start: quiet on errors, opens the popup when there is something to offer. */
export async function checkAppUpdate({ auto = false } = {}) {
  S.appUpdChecking = true; S.appUpdError = ""; redraw();
  try { S.appUpdate = await invoke("check_app_update"); }
  catch (er) { S.appUpdError = String(er); if (!auto) toast(er); }
  S.appUpdChecking = false; redraw();
  if (S.appUpdError) return;
  if (isPending()) { if (auto && !announced) { announced = true; openUpdatePopup(); } }
  else if (!auto) toast(S.appUpdate.latest ? `Version ${S.appUpdate.latest.version} is skipped — it is still available below` : "You are on the latest version");
}

/** Skip the release that is on offer now (a newer one is offered again). `undo` forgets the skip. */
export async function skipUpdate(undo = false) {
  const r = S.appUpdate && S.appUpdate.latest;
  if (!r) return;
  const tag = undo ? "" : r.tag;
  await call("skip_app_update", { tag });
  S.set.skipped_update = tag;
  S.appUpdate.skipped = !undo;
  redraw();
}
