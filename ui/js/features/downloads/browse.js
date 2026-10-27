// Downloads tab logic: browse / search GameBanana, queue files, local uploads.
import { invoke } from "../../core/tauri.js";
import { S } from "../../core/state.js";
import { $, esc, fmtSize } from "../../core/dom.js";
import { ic } from "../../core/icons.js";
import { modal, toast } from "../../core/feedback.js";
import { render } from "../../core/render.js";
import { cursorImport } from "../cursor/add.js";
import { fileNote } from "./view.js";

export async function loadBrowse(more) {
  if (S.browsing) return;
  S.browsing = true;
  if (!more) { S.loading = true; S.gbView = null; render(); }
  try {
    const page = more ? S.gbPage + 1 : 1, r = await invoke("gb_browse", { page });
    S.gbPage = page; S.gbList = more ? [...(S.gbList || []), ...r] : r; S.gbTitle = "Latest updates on GameBanana"; S.gbMode = "browse";
  } catch (er) { toast(er); if (!more) S.gbList = []; }
  S.loading = false; S.browsing = false; render();
}
export const fileStem = (n) => String(n).replace(/\.[^.]+$/, "");
export async function queueFiles(m, idxs, update) {
  try {
    for (const i of idxs) {
      const f = m.files[i];
      await invoke("download", { url: f.url, meta: { name: idxs.length > 1 ? `${m.name} - ${fileStem(f.name)}` : m.name, description: m.description, thumb: m.thumb, gb_id: m.id, author: m.author, updated: m.updated || 0, model: m.model || "Mod", update, audio: m.audio || "" } });
    }
    toast(update ? "Update added to the queue" : `${idxs.length} download${idxs.length > 1 ? "s" : ""} added to the queue`); render();
  } catch (er) { toast(er); }
}
export async function openFilePicker(m, update) {
  const multi = !update;
  await modal(`<h3>${update ? "Choose the file to replace the installed mod with" : "Choose the files to install"}</h3><p class="sub">${esc(m.name)}${multi ? " — each file is installed as its own mod" : ""}</p>
    <div class="list" style="max-height:50vh;overflow:auto;margin-bottom:12px">${m.files.map((f, i) => `<label class="row" style="cursor:pointer"><input type="${multi ? "checkbox" : "radio"}" name="pf" value="${i}" ${!multi && i === 0 ? "checked" : ""}><div class="nm"><b>${esc(f.name)}</b>${fileNote(f)}<small>${fmtSize(f.size)}</small></div></label>`).join("")}</div>
    <form method="dialog" class="btns">${multi ? `<button type="button" id="pfall">Select all / none</button>` : ""}<span class="grow"></span><button value="cancel" title="Cancel">${ic("x")}</button><button type="button" class="primary" id="pfgo">${ic("downloads")} ${update ? "Replace" : "Install"}</button></form>`, (d) => {
    const boxes = () => [...d.querySelectorAll('input[name="pf"]')];
    const all = d.querySelector("#pfall");
    if (all) all.onclick = () => { const on = boxes().some((b) => !b.checked); boxes().forEach((b) => (b.checked = on)); };
    d.querySelector("#pfgo").onclick = async () => {
      const idxs = boxes().filter((b) => b.checked).map((b) => +b.value);
      if (!idxs.length) return toast("Select at least one file");
      d.close("go"); await queueFiles(m, idxs, update);
    };
  });
}
export async function doFetch() {
  const v = ($("#gin") || {}).value?.trim();
  if (!v) { S.gbList = null; S.gbView = null; S.gbMode = ""; return render(); }
  S.loading = true; render();
  try {
    const r = await invoke("gb_lookup", { input: v });
    S.gbMode = "search";
    if (r.kind === "mod") S.gbView = r.item;
    else if (r.kind === "list") { S.gbView = null; S.gbList = r.items; S.gbTitle = `Results for "${v}"`; }
    else { await invoke("download", { url: r.url, meta: null }); toast("Direct file added to the queue"); S.dlSub = "queue"; }
  } catch (er) { toast(er); }
  S.loading = false; render();
  if ($("#gin")) $("#gin").value = v;
}
export function openLightbox(i) {
  const imgs = S.gbView.images; let k = i;
  modal(`<img class="big" id="lbimg" src="${esc(imgs[k])}"><div class="btns"><button id="lbp">‹</button><span class="sub" id="lbn"></span><button id="lbn2">›</button><button data-close title="Close">${ic("x")}</button></div>`, (d) => {
    const show = () => { d.querySelector("#lbimg").src = imgs[k]; d.querySelector("#lbn").textContent = `${k + 1} / ${imgs.length}`; };
    d.querySelector("#lbp").onclick = () => { k = (k - 1 + imgs.length) % imgs.length; show(); };
    d.querySelector("#lbn2").onclick = () => { k = (k + 1) % imgs.length; show(); };
    d.querySelector("[data-close]").onclick = () => d.close();
    show();
  }, true);
}
export async function addUploads(paths) {
  try {
    const items = await invoke("inspect_files", { paths });
    items.forEach((it) => S.up.push({ source: it.source, name: it.name, description: it.description || "", image: it.image || null }));
    S.tab = "downloads"; S.dlSub = "upload"; render();
  } catch (er) {
    // no .vpk inside: maybe it is a cursor pack
    try { await invoke("cursor_inspect", { paths }); return cursorImport(paths); } catch (_) { toast(er); }
  }
}
