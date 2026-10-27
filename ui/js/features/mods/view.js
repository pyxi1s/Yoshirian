// The Mods tab: list / card view of installed mods.
import { S, ui } from "../../core/state.js";
import { esc, bg, local } from "../../core/dom.js";
import { ic, ib } from "../../core/icons.js";
import { viewSeg } from "../../core/components.js";
import { warnings } from "./data.js";
import { isSoundMod } from "./sounds.js";
import { localDetail } from "./detail.js";

export function modsView() {
  if (S.modView) { const mv = S.mods.find((x) => x.yoshirian_id === S.modView); if (mv) return localDetail(mv); S.modView = null; }
  if (S.err) return `<div class="card"><h3>Game folder</h3><p>${esc(S.err)}</p><button class="primary" data-act="gosettings">${ic("settings")} Open Settings</button></div>`;
  const q = S.q.toLowerCase(), w = warnings();
  let n = 0;
  const tagOf = {};
  S.mods.forEach((m) => (tagOf[m.yoshirian_id] = m.is_enable ? `pak${String(++n).padStart(2, "0")}` : "disabled"));
  const on = S.mods.filter((m) => m.is_enable).length, off = S.mods.length - on;
  S.updCount = new Set(S.mods.filter((m) => S.updates[m.yoshirian_id]).map((m) => `${m.gb_id}:${m.gb_model}`)).size;
  const shown = S.mods.filter((m) => (!q || (m.name + " " + m.description + " " + m.author).toLowerCase().includes(q)) && (S.mf === "all" || (S.mf === "on") === m.is_enable));
  const canDrag = !q && S.mf === "all";
  const posOf = Object.fromEntries(S.mods.map((m, i) => [m.yoshirian_id, i + 1]));
  const ph = (m) => (m.image_path ? "" : String(m.is_enable ? +tagOf[m.yoshirian_id].slice(3) : posOf[m.yoshirian_id]).padStart(2, "0"));
  const play = (m) => (m.snd > 0 ? ib("playvpk", "play", "Preview the sound", `data-id="${m.yoshirian_id}"`, "sm") : m.gb_audio ? ib("playsnd", "play", "Preview the sound", `data-url="${esc(m.gb_audio)}"`, "sm") : "");
  const grip = (c = "") => (canDrag ? `<span class="grip ${c}" title="Drag to reorder">${ic("grip")}</span>` : "");
  const tags = (m) => `<span class="tag ${m.is_enable ? "on" : ""}">${tagOf[m.yoshirian_id]}</span>${isSoundMod(m) ? '<span class="tag">Sound</span>' : ""}${m.author ? `<span class="tag">${esc(m.author)}</span>` : ""}`;
  const upd = (m) => (S.updates[m.yoshirian_id] ? ib("modupd", "downloads", "Update available — click to update", `data-id="${m.yoshirian_id}"`, "sm primary") : "");
  const sw = (m) => `<label class="sw"><input type="checkbox" ${m.is_enable ? "checked" : ""} data-ch="toggle" data-id="${m.yoshirian_id}"><i></i></label>`;
  const warn = (m) => (w[m.yoshirian_id] ? `<span class="warn" title="${esc(w[m.yoshirian_id])}">⚠</span>` : "");
  const acts = (m) => upd(m) + ib("edit", "edit", "Edit", `data-id="${m.yoshirian_id}"`, "sm ghost") + ib("del", "trash", "Delete", `data-id="${m.yoshirian_id}"`, "sm del");
  let body;
  if (!shown.length) body = `<div class="empty">${S.mods.length ? "No mods match." : "No mods yet. Use Downloads or Add Local Mod."}</div>`;
  else if (ui.vm === "list")
    body = `<div class="list">${shown.map((m) => `<div class="row" data-id="${m.yoshirian_id}">${grip()}<div class="nm" data-act="modopen" data-id="${m.yoshirian_id}"><b>${esc(m.name)}</b><small>${esc(m.description || "No description")}</small><div>${tags(m)}</div></div>${play(m)}${warn(m)}${sw(m)}${acts(m)}</div>`).join("")}</div>`;
  else
    body = `<div class="grid">${shown.map((m) => `<div class="cardm" data-id="${m.yoshirian_id}">${grip("ov")}<span class="tag ov ${m.is_enable ? "on" : ""}">${tagOf[m.yoshirian_id]}</span><div class="img ${m.image_path ? "" : "ph"} ${m.is_enable ? "" : "off"}" data-act="modopen" data-id="${m.yoshirian_id}" ${bg(local(m.image_path))}>${m.snd > 0 ? `<button class="ib playov" title="Preview the sound" data-act="playvpk" data-id="${m.yoshirian_id}">${ic("play")}</button>` : m.gb_audio ? `<button class="ib playov" title="Preview the sound" data-act="playsnd" data-url="${esc(m.gb_audio)}">${ic("play")}</button>` : ph(m)}</div>
      <div class="body" data-act="modopen" data-id="${m.yoshirian_id}"><b title="${esc(m.description)}">${esc(m.name)}</b><div class="by">${m.author ? "By " + esc(m.author) : esc(m.description || "No description")}</div></div>
      <div class="foot">${sw(m)}${warn(m)}<span class="grow"></span>${acts(m)}</div></div>`).join("")}</div>`;
  const ft = [["all", `All (${S.mods.length})`], ["on", `Enabled (${on})`], ["off", `Disabled (${off})`]];
  return `<div class="head"><div><div class="titlerow"><h1>Mods</h1><span class="pillstat"><b>${on}</b>/${S.mods.length} mods</span><span class="pillstat"><b>${on}</b>/99 addon files</span></div>
    <div class="sub">Mods you have downloaded or installed</div></div><span class="grow"></span>
    ${S.updCount ? `<button class="primary" data-act="updall">${ic("downloads")} ${S.updCount} update${S.updCount > 1 ? "s" : ""}</button>` : ""}${ib("chkupd", "refresh", "Check for mod updates")}<button class="primary" data-act="addmod">${ic("upload")} Local Upload</button></div>
    <div class="toolbar"><div class="searchbox grow">${ic("search")}<input id="msearch" type="text" placeholder="Search mods" value="${esc(S.q)}" data-in="q"></div>
    <div class="tabs">${ft.map(([k, l]) => `<button data-act="mfilter" data-v="${k}" class="${S.mf === k ? "on" : ""}">${l}</button>`).join("")}</div>${viewSeg("vm")}</div>${body}`;
}
