// Feature: updating the app itself from GitHub Releases (no tab: title bar button, popup, Settings card).
import "./state.js";
import { invoke, listen } from "../../core/tauri.js";
import { S } from "../../core/state.js";
import { toast } from "../../core/feedback.js";
import { render } from "../../core/render.js";
import { checkAppUpdate, skipUpdate } from "./check.js";
import { openUpdatePopup, onUpdateProgress } from "./popup.js";

/** Change the repository the app updates from ("" = back to the default), then check it. */
async function setRepo(text) {
  try {
    const repo = await invoke("set_update_repo", { repo: text });
    S.set.update_repo = repo; S.set.skipped_update = ""; S.appUpdate = null; S.appUpdError = "";
    render();
    checkAppUpdate();
  } catch (er) { toast(er); render(); } // render() puts the old value back into the field
}

export { checkAppUpdate };

export default {
  id: "appupdate",
  init() { listen("app-update-progress", (ev) => onUpdateProgress(ev.payload)); },
  acts: {
    updopen: () => openUpdatePopup(),                    // title bar "Update" button
    updnow: () => openUpdatePopup({ start: true }),      // Settings "Update": straight to the download
    updcheck: () => checkAppUpdate(),
    updskip: () => skipUpdate(),
    updunskip: () => skipUpdate(true),
    updreset: () => setRepo(""),
  },
  changes: {
    updrepo: (d, el) => setRepo(el.value),
  },
};
