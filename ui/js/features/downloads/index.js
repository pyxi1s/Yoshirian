// Feature: Downloads tab — GameBanana search, download queue, local uploads, deep links.
import "./state.js";
import { invoke, listen, dialogApi } from "../../core/tauri.js";
import { S, activeDls } from "../../core/state.js";
import { toast, call, pickImage } from "../../core/feedback.js";
import { render, renderNav, renderChrome } from "../../core/render.js";
import { refresh } from "../mods/data.js";
import { preloadSoundsFor } from "../mods/sounds.js";
import { checkModUpdates } from "../mods/updates.js";
import { cursorsChanged } from "../cursor/data.js";
import { downloadsView, patchDlRow } from "./view.js";
import { loadBrowse, queueFiles, openFilePicker, doFetch, openLightbox, addUploads } from "./browse.js";
import { drainLinks } from "./links.js";

export { drainLinks, addUploads };

// progress events of the backend download queue
function onProgress(ev) {
  const d = ev.payload, prev = S.dls[d.id];
  S.dls[d.id] = { ...prev, ...d, total: d.total || (prev && prev.total) || 0 };
  if (d.state === "done") {
    if (d.kind === "cursor") { toast(`Added ${d.installed} cursor(s) to the Cursor library`); cursorsChanged(); }
    else { toast(`Installed "${d.name}" (disabled)`); refresh().then(() => checkModUpdates(false)); }
  }
  if (d.state === "error") toast(`Download failed: ${d.error}`);
  const onQueue = S.tab === "downloads" && S.dlSub === "queue";
  // same state as before: only the numbers changed, so patch the row instead of re-rendering (keeps its buttons clickable)
  if (onQueue && prev && prev.state === d.state && patchDlRow(S.dls[d.id])) return;
  if (onQueue) render(); else { renderNav(); renderChrome(); }
}

export default {
  id: "downloads",
  tab: { label: "Downloads", icon: "downloads", badge: activeDls },
  view: downloadsView,
  init() {
    listen("dl-progress", onProgress);
    listen("deeplink", drainLinks);
  },
  acts: {
    addmod: () => { S.tab = "downloads"; S.dlSub = "upload"; render(); },
    dsub: (d) => { S.dlSub = d.v; render(); },
    fetch: () => doFetch(),
    gmodel: (d) => { S.gm = d.v; render(); },
    gbopen: async (d) => { S.gbOpen = {}; S.loading = true; render(); try { S.gbView = await invoke("gb_mod", { id: +d.id, model: d.model || "Mod" }); await preloadSoundsFor(S.gbView); } catch (er) { toast(er); } S.loading = false; render(); },
    gbinst: async (d) => {
      try { await invoke("gb_install", { id: +d.id, model: d.model || "Mod" }); toast("Added to the download queue"); render(); }
      catch (er) {
        if (String(er) === "MULTI") { try { openFilePicker(await invoke("gb_mod", { id: +d.id, model: d.model || "Mod" }), false); } catch (e2) { toast(e2); } }
        else toast(er);
      }
    },
    gbtoggle: (d, el) => { S.gbOpen[d.k] = el.getAttribute("aria-expanded") !== "true"; render(); }, // fold / unfold a block of the detail page
    gbback: () => {
      S.gbView = null;
      if (S.gbFrom === "mods") { S.tab = "mods"; S.gbFrom = ""; }
      render();
    },
    lightbox: (d) => openLightbox(+d.i),
    gbdl: async (d) => {
      const m = S.gbView, upd = d.update === "1";
      if (m.files.length > 1) return openFilePicker(m, upd);
      await queueFiles(m, [0], upd);
    },
    gbmore: () => loadBrowse(true),
    gblatest: () => loadBrowse(),
    qclear: () => { for (const [k, d] of Object.entries(S.dls)) if (["done", "error", "cancelled"].includes(d.state)) delete S.dls[k]; render(); },
    dlpause: (d) => call("dl_pause", { id: d.id }),
    dlresume: (d) => call("dl_resume", { id: d.id }),
    dlcancel: (d) => call("dl_cancel", { id: d.id }),
    upick: async () => { const p = await dialogApi.open({ multiple: true, filters: [{ name: "Mods", extensions: ["vpk", ...S.exts] }] }); if (p && p.length) addUploads(p); },
    upx: (d) => { S.up.splice(+d.i, 1); render(); },
    upimg: async (d) => { const p = await pickImage(); if (p) { S.up[+d.i].image = p; render(); } },
    upimgx: (d) => { S.up[+d.i].image = null; render(); },
    uinstall: async () => {
      if (S.up.some((u) => !u.name.trim())) return toast("Every mod needs a name");
      try { await invoke("install_mods", { items: S.up.map((u) => ({ source: u.source, name: u.name.trim(), description: u.description, image: u.image })) }); toast(`Installed ${S.up.length} mod(s) (disabled)`); S.up = []; S.tab = "mods"; refresh(); } catch (er) { toast(er); }
    },
  },
  inputs: {
    gq: (d, el) => { S.gq = el.value; },
    up: (d, el) => { S.up[+d.i][d.f] = el.value; },
  },
};
