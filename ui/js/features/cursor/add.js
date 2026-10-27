// "Add cursors" dialogs (choose files / folder, confirm names and types).
import { invoke, dialogApi } from "../../core/tauri.js";
import { S } from "../../core/state.js";
import { esc, local } from "../../core/dom.js";
import { ic } from "../../core/icons.js";
import { modal, toast } from "../../core/feedback.js";
import { render } from "../../core/render.js";
import { CK } from "./view.js";
import { loadCursor } from "./data.js";

let addDlg = null;
export function openCursorAdd() {
  modal(`<h3>Add cursors</h3><div class="drop-zone" id="cadrop" style="margin:0 0 14px">Drag &amp; drop cursor images, folders or archives (${S.exts.join(", ")}) here</div>
    <div class="form" style="justify-content:center"><button class="primary" id="cafiles">${ic("upload")} Choose files</button><button id="cafolder">${ic("folder")} Choose folder</button></div>
    <form method="dialog" class="btns"><button value="cancel" title="Close">${ic("x")}</button></form>`, (d) => {
    addDlg = d;
    d.querySelector("#cafiles").onclick = async () => {
      const p = await dialogApi.open({ multiple: true, filters: [{ name: "Cursor images or archives", extensions: ["bmp", "png", "jpg", "jpeg", "gif", "webp", "ico", "cur", "tif", "tiff", ...S.exts] }] });
      if (p && p.length) { d.close(); cursorImport(p); }
    };
    d.querySelector("#cafolder").onclick = async () => { const p = await dialogApi.open({ directory: true }); if (p) { d.close(); cursorImport([p]); } };
  }).then(() => (addDlg = null));
}
export async function cursorImport(paths) {
  let items;
  try { items = await invoke("cursor_inspect", { paths }); } catch (er) { return toast(er); }
  S.tab = "cursor"; render(); if (!S.cur) loadCursor();
  const res = await modal(`<h3>Add cursors</h3><p class="sub">Files are converted to bmp at the exact size the game needs. Pick the type if it was guessed wrong.${[...new Set(items.map((x) => x.preset).filter(Boolean))].length ? `<br>Presets that will be created: ${[...new Set(items.map((x) => x.preset).filter(Boolean))].map(esc).join(", ")}` : ""}</p><div style="max-height:56vh;overflow:auto">${items.map((it, i) => `<div class="card" style="display:flex;gap:12px;align-items:center;padding:12px">
    <img class="cimg" src="${local(it.source)}" style="width:64px;height:64px" onerror="this.style.visibility='hidden'">
    <div style="flex:1;display:grid;gap:6px"><input type="text" data-i="${i}" data-f="name" value="${esc(it.name)}"><select data-i="${i}" data-f="kind">${CK.map(([k, l, sz]) => `<option value="${k}" ${it.kind === k ? "selected" : ""}>${l} (${sz})</option>`).join("")}</select></div></div>`).join("")}</div>
    <form method="dialog" class="btns"><button value="cancel" title="Cancel">${ic("x")}</button><button type="button" class="primary" id="cadd">${ic("plus")} Add ${items.length} cursor(s)</button></form>`, (d) => {
    d.querySelector("#cadd").onclick = async () => {
      d.querySelectorAll("[data-f]").forEach((el) => { items[+el.dataset.i][el.dataset.f] = el.value; });
      if (items.some((x) => !x.name.trim())) return toast("Every cursor needs a name");
      try { await invoke("cursor_add", { items }); d.close("ok"); } catch (er) { toast(er); }
    };
  }, true);
  if (res === "ok") { toast("Cursors added"); S.cv++; loadCursor(); }
}

/** Close the "Add cursors" dialog if it is open; returns whether it was. Used when files are dropped on the window. */
export function closeAddDialog() {
  const was = !!addDlg;
  if (addDlg) addDlg.close();
  return was;
}
