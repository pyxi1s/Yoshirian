// The window around the tabs: titlebar buttons, sidebar, volume, launch buttons, shortcuts, native file drop.
import { invoke, listen, win } from "./core/tauri.js";
import { S, ui, saveUi } from "./core/state.js";
import { $ } from "./core/dom.js";
import { ic } from "./core/icons.js";
import { toast, call } from "./core/feedback.js";
import { render, renderNav, goTab } from "./core/render.js";
import { runAct } from "./core/registry.js";
import { stopSound, applyVolume } from "./services/audio.js";
import { openProfiles } from "./features/mods/profiles.js";
import { addUploads } from "./features/downloads/index.js";
import { cursorImport, closeAddDialog } from "./features/cursor/add.js";

export function initShell() {
  $("#nav").addEventListener("click", (ev) => {
    const b = ev.target.closest("[data-tab]");
    if (b) goTab(b.dataset.tab);
  });
  document.addEventListener("keydown", (ev) => {
    if (ev.ctrlKey && ev.key.toLowerCase() === "k") { ev.preventDefault(); S.tab = "mods"; render(); const i = $("#msearch"); if (i) i.focus(); }
  });
  $("#pill").onclick = () => openProfiles();
  $("#collapse").onclick = () => { ui.collapsed = !ui.collapsed; saveUi(); renderNav(); };
  $("#openMods").onclick = () => call("open_folder", { kind: "addons" }).catch(() => {});
  $("#openGame").onclick = () => call("open_folder", { kind: "game" }).catch(() => {});
  $("#upd").onclick = () => runAct("ucheck");
  $("#reload").onclick = () => location.reload();

  // volume of sound previews
  const volUi = () => {
    const v = $("#vol"); v.value = Math.round(ui.volume * 100); v.style.setProperty("--p", v.value + "%");
    $("#volbtn").innerHTML = ic(ui.volume === 0 ? "mute" : "vol");
    applyVolume();
  };
  $("#vol").oninput = (ev) => { ui.volume = +ev.target.value / 100; if (ui.volume > 0) ui.last = ui.volume; saveUi(); volUi(); };
  $("#volstop").innerHTML = ic("stop");
  $("#volstop").onclick = () => stopSound();
  $("#volbtn").onclick = () => { ui.volume = ui.volume === 0 ? ui.last || 0.7 : 0; saveUi(); volUi(); };
  volUi();

  $("#btnMods").onclick = async () => { try { await invoke("launch_game", { vanilla: false }); toast("Launching Deadlock with mods…"); } catch (er) { toast(er); } };
  $("#btnVanilla").onclick = async () => { try { await invoke("launch_game", { vanilla: true }); toast("Launching Deadlock (vanilla)…"); } catch (er) { toast(er); } };
  $("#wmin").onclick = () => win.minimize();
  $("#wmax").onclick = () => win.toggleMaximize();
  $("#wclose").onclick = () => win.close();

  // native file drop (real paths from Tauri): cursor files go to the Cursor tab / dialog, everything else is a mod
  listen("tauri://drag-enter", () => { $("#drop").classList.add("on"); const z = $("#cadrop"); if (z) z.style.borderColor = "var(--accent)"; });
  listen("tauri://drag-leave", () => { $("#drop").classList.remove("on"); const z = $("#cadrop"); if (z) z.style.borderColor = ""; });
  listen("tauri://drag-drop", (ev) => {
    $("#drop").classList.remove("on");
    const p = ev.payload.paths || [];
    const wasAdd = closeAddDialog();
    if (p.length) (S.tab === "cursor" || wasAdd ? cursorImport(p) : addUploads(p));
  });
}
