// Rendering: sidebar, titlebar status and the active tab's view.
import { S, ui, activeDls } from "./state.js";
import { $, esc } from "./dom.js";
import { ic } from "./icons.js";
import { tabFeatures, tabFeature } from "./registry.js";

const afterRender = [];
/** Register a function that runs after every render (e.g. to mount waveform canvases). */
export const onAfterRender = (fn) => afterRender.push(fn);

export function renderNav() {
  $("#nav").innerHTML = tabFeatures().map((f) => {
    const n = f.tab.badge ? f.tab.badge() : 0;
    return `<button data-tab="${f.id}" class="${S.tab === f.id ? "on" : ""}" title="${f.tab.label}">${ic(f.tab.icon)}<span class="lbl">${f.tab.label}</span>${n ? `<span class="badge">${n}</span>` : ""}</button>`;
  }).join("");
  $("#collapse").innerHTML = `${ic(ui.collapsed ? "expand" : "collapse")}<span class="lbl">Collapse menu</span>`;
  document.body.classList.toggle("collapsed", !!ui.collapsed);
}

export function renderChrome() {
  const on = S.mods.filter((m) => m.is_enable).length;
  $("#pill").innerHTML = `${ic("user")}<b>${esc(ui.activeProfile || "Profiles")}</b>${ic("chev")}`;
  $("#btnVanilla").innerHTML = `<span class="ico">${ic("undo")}</span><span class="lbl">Launch vanilla</span>`;
  $("#btnMods").innerHTML = `${ic("play")}<span class="lbl">Launch with mods · ${on}</span>`;
  $("#openMods").innerHTML = `${ic("folder")} Mods folder`;
  $("#openGame").innerHTML = `${ic("folder")} Game folder`;
  // "Update" is only there while an update is on offer (not skipped)
  const au = S.appUpdate, pend = !!(au && au.latest && !au.skipped), ub = $("#upd");
  ub.hidden = !pend;
  ub.classList.toggle("crit", pend && au.latest.critical);
  ub.title = pend ? `Version ${au.latest.version} is available${au.latest.critical ? " (critical fix)" : ""}` : "";
  ub.innerHTML = `${ic("up")}<span>Update</span>`;
  $("#reload").innerHTML = ic("refresh");
  const ok = !!S.set.game_root && !S.err;
  $("#tstatus").innerHTML = `<span class="st ${ok ? "ok" : "bad"}" title="${ok ? "Game folder found" : "Game folder missing"}">${ic("folder")}</span><span class="st ${S.patched ? "ok" : ""}" title="gameinfo.gi: ${S.patched ? "patched (mods enabled)" : "vanilla"}">${ic("check")}</span>${activeDls() ? `<span class="st ok" title="${activeDls()} download(s) in progress">${ic("downloads")}</span>` : ""}`;
}

export function render() {
  renderNav();
  renderChrome();
  const C = $("#content"), st = C.scrollTop;
  const keep = document.activeElement && document.activeElement.id;
  C.innerHTML = tabFeature(S.tab).view();
  C.scrollTop = S.rt === S.tab ? st : 0;
  afterRender.forEach((fn) => fn());
  S.rt = S.tab;
  if (keep && $("#" + keep)) { const e = $("#" + keep); e.focus(); if (e.type === "text" && e.setSelectionRange) e.setSelectionRange(e.value.length, e.value.length); }
}

/** Switch tab: beforeEnter -> render -> onEnter (the place for async loading). */
export function goTab(id) {
  const f = tabFeature(id);
  if (f.tab.beforeEnter) f.tab.beforeEnter();
  S.tab = id;
  render();
  if (f.tab.onEnter) f.tab.onEnter();
}
