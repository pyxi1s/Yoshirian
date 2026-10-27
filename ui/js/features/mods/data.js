// Loading the mod list from the backend.
import { invoke } from "../../core/tauri.js";
import { S } from "../../core/state.js";
import { render } from "../../core/render.js";
import { SND } from "../../services/audio.js";
import { loadSounds, preloadSoundsFor } from "./sounds.js";

export async function refresh() {
  S.err = "";
  S.sndLists = {};
  for (const k of [...SND.keys()]) if (k.startsWith("vpk:")) SND.delete(k);
  if (!S.set.game_root) { S.err = "Game folder is not set."; return render(); }
  try {
    S.mods = await invoke("list_mods");
    S.profiles = await invoke("list_profiles");
    S.patched = await invoke("gameinfo_status");
    // keep sound panels of an open detail page working after the lists were cleared
    if (S.modView) { const mv = S.mods.find((x) => x.yoshirian_id === S.modView); if (mv && mv.snd > 0) await loadSounds(S.modView); }
    if (S.gbView) await preloadSoundsFor(S.gbView);
  } catch (er) { S.err = String(er); }
  render();
}
export function warnings() {
  const w = {}, by = Object.fromEntries(S.mods.map((m) => [m.name.toLowerCase(), m]));
  for (const m of S.mods.filter((x) => x.is_enable)) for (const r of m.requires || []) {
    const d = by[r.toLowerCase()];
    if (!d) w[m.yoshirian_id] = `Requires "${r}" (not installed)`;
    else if (!d.is_enable) w[m.yoshirian_id] = `Requires "${r}" to be enabled`;
    else if (d.order > m.order) w[m.yoshirian_id] = `"${r}" should load before this mod`;
  }
  return w;
}
