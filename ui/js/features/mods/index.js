// Feature: Mods tab — installed mods, profiles, sounds inside mods, update tracking.
import "./state.js";
import { invoke } from "../../core/tauri.js";
import { S } from "../../core/state.js";
import { toast, call, askConfirm } from "../../core/feedback.js";
import { render } from "../../core/render.js";
import { runAct } from "../../core/registry.js";
import { refresh } from "./data.js";
import { loadSounds, soundActs } from "./sounds.js";
import { updateActs } from "./updates.js";
import { modsView } from "./view.js";
import { openEdit } from "./edit.js";
import { initReorder } from "./reorder.js";

export default {
  id: "mods",
  tab: { label: "Mods", icon: "mods", badge: () => S.mods.length, beforeEnter: () => { S.modView = null; } },
  view: modsView,
  init: initReorder,
  acts: {
    edit: (d) => openEdit(d.id),
    del: async (d) => { const m = S.mods.find((x) => x.yoshirian_id === d.id); if (await askConfirm(`Delete "${m.name}" permanently?`)) { await call("delete_mod", { id: d.id }); S.modView = null; refresh(); } },
    mfilter: (d) => { S.mf = d.v; render(); },
    modback: () => { S.modView = null; render(); },
    modopen: async (d) => {
      const m = S.mods.find((x) => x.yoshirian_id === d.id); if (!m) return;
      if (m.gb_id) {
        // installed from GameBanana: open its store page (downloads feature)
        S.gbFrom = "mods"; S.tab = "downloads"; S.dlSub = "download"; S.gbView = null; render();
        await runAct("gbopen", { id: m.gb_id, model: m.gb_model || "Mod" });
        if (S.gbView) return;
        S.gbFrom = ""; S.tab = "mods";
      }
      S.modView = m.yoshirian_id;
      if (m.snd > 0) await loadSounds(m.yoshirian_id);
      render();
    },
    rereadys: async (d) => { try { const ok = await invoke("reread_meta", { id: d.id }); toast(ok ? "Loaded from the .yoshirian file" : "No .yoshirian file found in this mod"); await refresh(); } catch (er) { toast(er); } },
    ...soundActs,
    ...updateActs,
  },
  changes: {
    toggle: async (d, el) => { try { await invoke("toggle_mod", { id: d.id, enable: el.checked }); } catch (er) { toast(er); } refresh(); },
  },
  inputs: {
    q: (d, el) => { S.q = el.value; render(); },
  },
};
