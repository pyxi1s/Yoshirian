// Mod update tracker (notify only) and the update actions.
import { invoke } from "../../core/tauri.js";
import { S } from "../../core/state.js";
import { toast, call } from "../../core/feedback.js";
import { render } from "../../core/render.js";
import { runAct } from "../../core/registry.js";
import { refresh } from "./data.js";

export async function checkModUpdates(manual) {
  if (!S.set.game_root) return;
  try {
    const r = await invoke("check_mod_updates");
    S.updates = Object.fromEntries(r.map((x) => [x.id, x.remote]));
    render();
    if (r.length) toast(`${r.length} mod update${r.length > 1 ? "s" : ""} available`);
    else if (manual) toast("All mods are up to date");
  } catch (er) { if (manual) toast(er); }
  S.updChecked = true;
  if (S.tab === "downloads") render();
}

export const updateActs = {
  // Skip the update that is available now. A newer one notifies again. Applies to all files of the same GameBanana page.
  modskip: async (d) => {
    const m = S.mods.find((x) => x.yoshirian_id === d.id), remote = S.updates[d.id];
    if (!m || !remote) return;
    await call("skip_mod_update", { id: d.id, remote });
    for (const x of S.mods) if (x.gb_id === m.gb_id && (x.gb_model || "Mod") === (m.gb_model || "Mod")) delete S.updates[x.yoshirian_id];
    toast("Update skipped — you will be told when a newer one is released");
    await refresh();
  },
  modunskip: async (d) => {
    await call("skip_mod_update", { id: d.id, remote: 0 });
    await refresh();
    await checkModUpdates(false);
  },
  chkupd: () => checkModUpdates(true),
  modupd: async (d) => {
    try { await invoke("update_mod", { id: d.id }); toast("Update added to the download queue"); render(); }
    catch (er) {
      if (String(er) === "MULTI") { const m = S.mods.find((x) => x.yoshirian_id === d.id); toast("This mod has several files — pick one and use Replace installed"); S.tab = "downloads"; S.dlSub = "download"; render(); await runAct("gbopen", { id: m.gb_id, model: m.gb_model || "Mod" }); }
      else toast(er);
    }
  },
  updall: async () => {
    const seen = new Set(); let n = 0, multi = 0;
    for (const id of Object.keys(S.updates)) {
      const m = S.mods.find((x) => x.yoshirian_id === id); if (!m) continue;
      const key = `${m.gb_id}:${m.gb_model}`; if (seen.has(key)) continue; seen.add(key);
      try { await invoke("update_mod", { id }); n++; } catch (er) { if (String(er) === "MULTI") multi++; else toast(er); }
    }
    toast(`Queued ${n} update(s)${multi ? `, ${multi} need a manual choice (open the mod page)` : ""}`); render();
  },
};
