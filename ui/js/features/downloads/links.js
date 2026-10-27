// yoshirian:// deep links (GameBanana one-click install).
import { invoke } from "../../core/tauri.js";
import { S, ui } from "../../core/state.js";
import { toast, askConfirm } from "../../core/feedback.js";
import { render } from "../../core/render.js";
import { runAct } from "../../core/registry.js";

export async function drainLinks() {
  for (const link of await invoke("take_links")) {
    let k;
    try { k = await invoke("link_kind", { link }); } catch (_) { continue; }
    if (k.kind === "invalid") { toast("Only GameBanana links are supported"); continue; }
    if (k.kind === "open") { S.tab = "downloads"; S.dlSub = "download"; render(); await runAct("gbopen", { id: k.id, model: k.model }); continue; }
    if (ui.askLink && !(await askConfirm(`Install from GameBanana?\n\n${k.url}`))) continue;
    try { await invoke("process_link", { link }); S.tab = "downloads"; S.dlSub = "queue"; render(); } catch (er) { toast(er); }
  }
}
