// Shared application state.
//   S  : runtime state (never persisted). Each feature adds its own fields in features/<name>/state.js.
//   ui : view preferences, theme and toggles. Persisted in localStorage under "yoshi.ui4".
import { newTheme } from "./theme-data.js";

export const S = {
  tab: "mods", rt: "", err: "", version: "",
  appUpdate: null,   // answer of check_app_update: { current, repo, latest, skipped, installer } (null = not checked)
  defaultRepo: "",   // the repository the app updates from unless the user changes it
  exts: ["zip", "7z", "rar", "tar", "gz", "tgz", "bz2", "xz", "zst"], // archive types the backend can open (see archive_exts)
  // settings stored by the backend (settings.json)
  set: { auto_detect: true, gameinfo_auto: true, external_editor: "", launch_options: "", update_repo: "", skipped_update: "", auto_update_check: true, auto_mod_update_check: true, auto_cursor_preset: true, debug: false, game_root: null },
  // shared by several features
  mods: [], profiles: [], patched: false,
  dls: {}, // download queue, filled by the "dl-progress" event
};

export const ui = (() => {
  let u = {};
  try { u = JSON.parse(localStorage.getItem("yoshi.ui4") || "{}"); } catch (_) {}
  const t = newTheme();
  u.theme = { ...t, ...(u.theme || {}), vars: { ...t.vars, ...((u.theme || {}).vars || {}) } };
  return { vm: "list", vd: "card", volume: 0.7, activeProfile: null, collapsed: false, saved: {}, askLink: true, lt: ["-novid", "-console", "-windowed", "-noborder", "-high"], at: ["cl_showfps 1", "fps_max 0"], ...u };
})();
export const saveUi = () => localStorage.setItem("yoshi.ui4", JSON.stringify(ui));

/** Number of downloads that are queued / running / paused / installing (shown on the sidebar and titlebar). */
export const activeDls = () => Object.values(S.dls).filter((d) => ["queued", "downloading", "paused", "installing"].includes(d.state)).length;
