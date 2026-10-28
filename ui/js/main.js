// Entry point: register features, wire the shell, load data.
import { invoke } from "./core/tauri.js";
import { S } from "./core/state.js";
import { $ } from "./core/dom.js";
import { applyTheme } from "./core/theme.js";
import { register, initFeatures } from "./core/registry.js";
import { bindContentEvents } from "./core/events.js";
import { onAfterRender } from "./core/render.js";
import { mountWaves } from "./services/audio.js";
import { FEATURES } from "./features/index.js";
import { initShell } from "./shell.js";
import { refresh } from "./features/mods/data.js";
import { checkModUpdates } from "./features/mods/updates.js";
import { drainLinks } from "./features/downloads/index.js";
import { checkAppUpdate } from "./features/appupdate/index.js";

register(FEATURES);
onAfterRender(mountWaves);
bindContentEvents();
initFeatures();
initShell();

(async () => {
  applyTheme();
  S.set = await invoke("get_settings");
  const info = await invoke("get_info");
  S.version = info.version;
  S.defaultRepo = info.default_update_repo;
  S.exts = await invoke("archive_exts").catch(() => S.exts);
  $("#ver").textContent = "v" + info.version;
  await refresh();
  drainLinks();
  if (S.set.auto_mod_update_check) checkModUpdates(false);
  if (S.set.auto_update_check) checkAppUpdate({ auto: true }); // every start; the popup opens only for a release / hotfix that was not skipped
})();
