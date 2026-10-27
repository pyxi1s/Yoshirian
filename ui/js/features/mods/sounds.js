// Sounds inside installed mods (vpk files) + the play buttons used by mods and GameBanana pages.
import { invoke } from "../../core/tauri.js";
import { S } from "../../core/state.js";
import { esc } from "../../core/dom.js";
import { ib } from "../../core/icons.js";
import { toast } from "../../core/feedback.js";
import { render } from "../../core/render.js";
import { playSound, waveHtml } from "../../services/audio.js";

export const isSoundMod = (m) => m.gb_model === "Sound" || m.snd > 0;
export async function loadSounds(id) {
  if (S.sndLists[id] === undefined) { try { S.sndLists[id] = await invoke("vpk_sounds", { id }); } catch (_) { S.sndLists[id] = []; } }
  return S.sndLists[id];
}
export async function preloadSoundsFor(m) {
  for (const x of S.mods.filter((x) => x.gb_id === m.id && (x.gb_model || "Mod") === (m.model || "Mod") && x.snd > 0)) await loadSounds(x.yoshirian_id);
}
export function soundPanel(id, title) {
  const list = S.sndLists[id] || [];
  if (!list.length) return "";
  const sel = Math.min(S.sndSel[id] || 0, list.length - 1), key = (e) => `vpk:${id}:${e.path}`;
  return `<div class="card"><h3>${esc(title)} <small class="sub">${list.length} sound${list.length > 1 ? "s" : ""} in the file</small></h3>${waveHtml(key(list[sel]))}
    <div class="list" style="margin-top:12px;max-height:260px;overflow:auto">${list.map((e, i) => `<div class="row click" data-act="sndsel" data-id="${id}" data-i="${i}" style="padding:5px 12px"><span class="radio ${i === sel ? "on" : ""}"></span><div class="nm"><b style="font-weight:500">${esc(e.path.split("/").pop())}</b><small>${esc(e.path)}</small></div>${ib("playsnd", "play", "Play", `data-url="${esc(key(e))}"`, "sm")}</div>`).join("")}</div></div>`;
}

export const soundActs = {
  playvpk: async (d) => {
    const list = await loadSounds(d.id);
    if (!list.length) return toast("No sounds found in this file");
    playSound(`vpk:${d.id}:${list[Math.min(S.sndSel[d.id] || 0, list.length - 1)].path}`);
  },
  sndsel: (d) => { S.sndSel[d.id] = +d.i; render(); const l = S.sndLists[d.id]; if (l && l[+d.i]) playSound(`vpk:${d.id}:${l[+d.i].path}`); },
  // url = remote preview, "vpk:<id>:<path>" = sound inside a mod, or data-id/data-model = look the preview up on GameBanana
  playsnd: async (d) => {
    let url = d.url;
    const mm = url && url.match(/^vpk:([^:]+):(.*)$/);
    if (mm) { const l = S.sndLists[mm[1]], i = l ? l.findIndex((e) => e.path === mm[2]) : -1; if (i >= 0 && S.sndSel[mm[1]] !== i) { S.sndSel[mm[1]] = i; render(); } }
    if (!url && d.id) {
      try { url = (await invoke("gb_mod", { id: +d.id, model: d.model || "Sound" })).audio || ""; } catch (er) { return toast(er); }
      if (!url) return toast("This sound has no preview");
    }
    playSound(url);
  },
};
