// Detail page of an installed mod.
import { S } from "../../core/state.js";
import { esc, fmtSize, bg, local, linkify } from "../../core/dom.js";
import { ib } from "../../core/icons.js";
import { isSoundMod, soundPanel } from "./sounds.js";

/** "Update available" with Update / Skip, or a reminder that an update was skipped (with Undo). */
export const updateBanner = (m) => {
  const id = m.yoshirian_id;
  if (S.updates[id]) return `<div class="card upd-banner"><b>Update available</b><span class="sub">A newer version is on GameBanana.</span><span class="grow"></span><button class="primary" data-act="modupd" data-id="${id}">Update</button><button data-act="modskip" data-id="${id}" title="You will still be told about newer updates">Skip this update</button></div>`;
  if (m.gb_id && m.gb_skipped > m.gb_updated) return `<div class="sub" style="margin:10px 2px">You skipped an update of this mod. <a href="#" data-act="modunskip" data-id="${id}">Undo</a></div>`;
  return "";
};

export function localDetail(m) {
  const id = m.yoshirian_id, sound = isSoundMod(m);
  const pak = m.is_enable ? "pak" + String(S.mods.filter((x) => x.is_enable && x.order <= m.order).length).padStart(2, "0") : "disabled";
  const thumb = !sound && m.image_path ? `<div class="thumb" ${bg(local(m.image_path), "width:360px;max-width:100%;aspect-ratio:16/9")}></div>` : "";
  const src = m.gb_id ? `GameBanana · <a href="#" data-act="openlink" data-url="https://gamebanana.com/${(m.gb_model || "Mod") === "Sound" ? "sounds" : "mods"}/${m.gb_id}">#${m.gb_id}</a>` : "Local upload";
  return `${ib("modback", "back", "Back")}
  <div class="card" style="margin-top:14px"><div style="display:flex;gap:20px;flex-wrap:wrap">${thumb}
    <div style="flex:1;min-width:260px"><h1>${esc(m.name)}</h1>
      <div class="stats" style="margin:4px 0 10px">${sound ? '<span class="tag">Sound</span>' : ""}<span class="tag ${m.is_enable ? "on" : ""}">${pak}</span>${m.author ? `<span class="tag">${esc(m.author)}</span>` : ""}</div>
      <div class="kv" style="margin-bottom:12px"><span>Source</span><span>${src}</span><span>File</span><span>${esc(m.current_file)}</span><span>Size</span><span>${fmtSize(m.size)}</span><span>Load order</span><span>${m.order}</span>${(m.requires || []).length ? `<span>Requires</span><span>${esc(m.requires.join(", "))}</span>` : ""}</div>
      <div class="form"><label class="sw"><input type="checkbox" ${m.is_enable ? "checked" : ""} data-ch="toggle" data-id="${id}"><i></i></label><span class="sub">${m.is_enable ? "Enabled" : "Disabled"}</span><span class="grow"></span>${m.has_ys ? ib("rereadys", "refresh", "Reload name, description and image from the .yoshirian file inside this mod", `data-id="${id}"`) : ""}${ib("edit", "edit", "Edit", `data-id="${id}"`)}${ib("del", "trash", "Delete", `data-id="${id}"`, "del")}</div></div></div></div>
  ${updateBanner(m)}
  ${sound ? soundPanel(id, "Preview") : ""}
  <div class="card"><h3>Description</h3><div style="white-space:pre-wrap;user-select:text">${m.description ? linkify(m.description) : '<span class="sub">No description.</span>'}</div></div>`;
}
