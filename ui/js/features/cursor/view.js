// The Cursor tab.
import { S } from "../../core/state.js";
import { esc, local } from "../../core/dom.js";
import { ic, ib } from "../../core/icons.js";

export const CK = [["cursor", "Default cursor", "48×58"], ["ping", "Ping", "48×58"], ["shop", "Shop", "60×60"], ["commend", "Commend", "80×80"]];
export const kLabel = (k) => (CK.find((x) => x[0] === k) || [k, k])[1];
export const cimg = (p, w, h) => (p ? `<img class="cimg" src="${local(p)}?v=${S.cv}" style="width:${w * 2}px;height:${h * 2}px" onerror="this.style.visibility='hidden'">` : `<div class="cimg" style="width:${w * 2}px;height:${h * 2}px"></div>`);
export function cursorView() {
  const head = `<div class="head"><div><h1>Cursor</h1><div class="sub">Replace the in-game cursors. One cursor per type; the originals are backed up automatically.</div></div><span class="grow"></span>
    ${ib("curreapply", "refresh", "Re-apply the cursors in use (after a game update)")}${ib("curreset", "undo", "Restore all original cursors")}<button class="primary" data-act="curadd">${ic("upload")} Add cursors</button></div>`;
  if (!S.set.game_root) return head + `<div class="card"><p>Set the game folder first.</p><button class="primary" data-act="gosettings">${ic("settings")} Open Settings</button></div>`;
  const c = S.cur;
  if (!c) return head + `<div class="empty">Loading…</div>`;
  if (c.error) return head + `<div class="card"><p>${esc(c.error)}</p></div>`;
  const dims = Object.fromEntries(c.kinds.map((k) => [k.kind, k]));
  const slots = CK.map(([k, l, sz]) => {
    const i = dims[k], act = c.active[k] || "", mods = c.mods.filter((m) => m.kind === k);
    return `<div class="card slot">${cimg(i.game_path, i.w, i.h)}<div style="flex:1;min-width:0"><b>${l}</b> <small class="sub">${sz} · ${esc(i.file)}</small>
      <div class="form" style="margin:8px 0 0"><select data-ch="cuse" data-k="${k}" style="flex:1"><option value="">Original</option>${mods.map((m) => `<option value="${m.id}" ${act === m.id ? "selected" : ""}>${esc(m.name)}</option>`).join("")}</select></div></div></div>`;
  }).join("");
  const presets = `<h2>Presets</h2><div class="compact">${c.profiles.length ? `<div class="list" style="margin-bottom:8px">${c.profiles.map((p) => { const act = CK.every(([k]) => (p.map[k] || null) === (c.active[k] || null)), n = CK.filter(([k]) => p.map[k]).length, names = CK.map(([k, l]) => { const m = c.mods.find((x) => x.id === p.map[k]); return `${l}: ${m ? m.name : "Original"}`; }).join("\n");
      return `<div class="row"><span class="radio ${act ? "on" : ""}" title="${act ? "In use" : "Not in use — click to apply"}" data-act="cpapply" data-n="${esc(p.name)}"></span><div class="nm" style="display:flex;gap:10px;align-items:baseline;overflow:hidden"><b style="flex:none;max-width:60%;overflow:hidden;text-overflow:ellipsis">${esc(p.name)}</b><small title="${esc(names)}">${n}/4 types</small></div>
      ${ib("cpapply", "play", "Apply this preset", `data-n="${esc(p.name)}"`, "sm primary")}${ib("cpupdate", "refresh", "Update with the cursors in use now", `data-n="${esc(p.name)}"`, "sm")}${ib("cpdel", "trash", "Delete", `data-n="${esc(p.name)}"`, "sm danger")}</div>`; }).join("")}</div>` : `<p class="sub">No presets yet. Pick a cursor for each type, then save the set as a preset.</p>`}
    <div class="form"><input type="text" id="cpname" placeholder="New preset name" data-enter="cpadd">${ib("cpadd", "plus", "Save current cursors as a preset", "", "sm primary")}</div></div>`;
  const q = S.cq.toLowerCase();
  const shown = c.mods.filter((m) => (S.ck === "all" || m.kind === S.ck) && (!q || m.name.toLowerCase().includes(q)));
  const lib = shown.length ? `<div class="grid">${shown.map((m) => {
    const i = dims[m.kind], on = c.active[m.kind] === m.id;
    return `<div class="cardm"><div class="cpv">${cimg(m.path, i.w, i.h)}</div><div class="body"><b>${esc(m.name)}</b><div class="by">${kLabel(m.kind)}${on ? " · in use" : ""}</div></div>
      <div class="foot"><button class="sm ${on ? "primary" : ""}" data-act="cuse" data-id="${m.id}" data-k="${m.kind}">${on ? "In use" : "Use"}</button><span class="grow"></span>${ib("curedit", "edit", "Rename", `data-id="${m.id}"`, "sm ghost")}${ib("curdel", "trash", "Delete", `data-id="${m.id}"`, "sm del")}</div></div>`;
  }).join("")}</div>` : `<div class="empty">${c.mods.length ? "No cursors match." : "No cursors yet. Add images or archives (bmp, png, jpg, gif, webp, ico, zip, rar, 7z)."}</div>`;
  const tabs = [["all", "All"], ...CK.map(([k, l]) => [k, l])];
  return head + `<h2>In use</h2><div class="slots">${slots}</div><div style="margin-bottom:26px">${presets}</div>
    <h2>Library</h2><div class="toolbar"><div class="searchbox grow">${ic("search")}<input id="csearch" type="text" placeholder="Search cursors" value="${esc(S.cq)}" data-in="cq"></div>
    <div class="tabs">${tabs.map(([k, l]) => `<button data-act="ckind" data-v="${k}" class="${S.ck === k ? "on" : ""}">${l}</button>`).join("")}</div></div>${lib}`;
}
