// Feature: Settings tab — game folder, gameinfo.gi, launch options, autoexec, appearance, updates, logs.
import "./state.js";
import { invoke, dialogApi } from "../../core/tauri.js";
import { S, ui, saveUi } from "../../core/state.js";
import { $ } from "../../core/dom.js";
import { toast, call, askConfirm, askText, pickImage } from "../../core/feedback.js";
import { render, goTab } from "../../core/render.js";
import { applyTheme, resetFontCache } from "../../core/theme.js";
import { PRESETS, VARS, newTheme } from "../../core/theme-data.js";
import { refresh } from "../mods/data.js";
import { settingsView, SUBS, logHtml } from "./sections.js";
import { setList } from "./editors.js";
import { hlCode } from "./highlight.js";

const saveSet = () => call("save_settings", { settings: S.set });
export const enterSettings = () => goTab("settings");

// Load what the settings page shows (editor texts, patch status, logs, diagnostics) after it was rendered.
async function loadSettingsData() {
  for (const k of ["gameinfo", "autoexec"]) if (S.txt[k] === undefined) { try { S.txt[k] = await invoke("read_text_file", { kind: k }); } catch (_) { S.txt[k] = ""; } }
  S.patched = await invoke("gameinfo_status").catch(() => false);
  S.logs = await invoke("get_logs").catch(() => []);
  S.info = await invoke("get_info").catch(() => null);
  render();
}

const acts = {
  gosettings: () => enterSettings(),
  ssub: (d) => { S.sub = d.v; document.querySelectorAll(".subnav button").forEach((b) => b.classList.toggle("on", b.dataset.v === d.v)); const el = $("#sec-" + d.v); if (el) el.scrollIntoView({ behavior: "smooth", block: "start" }); },
  sdetect: async () => { const p = await call("detect_root"); if (!p) return toast("Deadlock was not found automatically"); await call("set_root", { path: p }); S.set.game_root = p; toast("Game folder detected"); S.txt = {}; refresh().then(() => S.tab === "settings" && enterSettings()); },
  spick: async () => { const p = await dialogApi.open({ directory: true }); if (p) { await call("set_root", { path: p }); S.set.game_root = p; toast("Game folder saved"); S.txt = {}; refresh().then(() => S.tab === "settings" && enterSettings()); } },
  dopen: (d) => call("open_folder", { kind: d.k }),
  gpatch: async (d) => { const r = await call("patch_gameinfo_cmd", { force: d.force === "1" }); toast(r ? "gameinfo.gi patched" : "Already patched"); S.txt.gameinfo = undefined; enterSettings(); },
  grestore: async () => { await call("restore_gameinfo"); toast("Restored to vanilla"); S.txt.gameinfo = undefined; enterSettings(); },
  editorpick: async () => { const p = await dialogApi.open({ filters: [{ name: "Programs", extensions: ["exe"] }] }); if (p) { S.set.external_editor = p; await saveSet(); render(); } },
  tsave: async (d) => { await call("write_text_file", { kind: d.kind, content: S.txt[d.kind] ?? "" }); toast("Saved"); if (d.kind === "gameinfo") { S.patched = await invoke("gameinfo_status"); render(); } },
  treload: async (d) => { S.txt[d.kind] = undefined; await enterSettings(); },
  text: (d) => call("open_external", { kind: d.kind }),
  tg: async (d) => {
    const t = setList(d.kind)[+d.i];
    if (d.kind === "launch") {
      let o = " " + (S.set.launch_options || "").trim() + " ";
      o = o.includes(" " + t + " ") ? o.replace(" " + t + " ", " ") : o + t + " ";
      S.set.launch_options = o.replace(/\s+/g, " ").trim();
      await saveSet();
    } else {
      let lines = (S.txt.autoexec || "").split(/\r?\n/);
      lines = lines.map((l) => l.trim()).includes(t.trim()) ? lines.filter((l) => l.trim() !== t.trim()) : [...lines.filter((l, i, a) => i < a.length - 1 || l.trim() !== ""), t];
      S.txt.autoexec = lines.join("\n").replace(/\n*$/, "\n");
      await call("write_text_file", { kind: "autoexec", content: S.txt.autoexec });
    }
    render();
  },
  tgx: (d, el, ev) => { ev.stopPropagation(); setList(d.kind).splice(+d.i, 1); saveUi(); render(); },
  tgadd: (d) => {
    const inp = $("#tgnew-" + d.kind), v = (inp.value || "").trim();
    if (!v) return;
    if (!setList(d.kind).includes(v)) setList(d.kind).push(v);
    saveUi(); render();
  },
  ucheck: async () => { try { const v = await invoke("check_update"); toast(v ? `New version available: ${v}` : "You are on the latest version"); } catch (er) { toast(er); } },
  lrefresh: async () => { S.logs = await invoke("get_logs"); render(); },
  lclear: async () => { await invoke("clear_logs"); S.logs = []; render(); },
  dreload: () => refresh().then(() => toast("Reloaded")),
  dcopy: async () => { const i = await invoke("get_info"); await navigator.clipboard.writeText(JSON.stringify({ info: i, settings: S.set, mods: S.mods.length, patched: S.patched }, null, 2)); toast("Diagnostics copied"); },
  dreset: async () => { if (await askConfirm("Reset all appearance settings and saved themes?")) { ui.theme = newTheme(); ui.saved = {}; applyTheme(); render(); } },
  theme: (d) => { const i = d.k.indexOf(":"), k = d.k.slice(0, i), n = d.k.slice(i + 1); ui.theme = k === "p" ? newTheme(n) : JSON.parse(JSON.stringify(ui.saved[n])); resetFontCache(); applyTheme(); render(); },
  thsave: async () => { const n = await askText("Theme name", ui.theme.name); if (n) { ui.saved[n] = JSON.parse(JSON.stringify({ ...ui.theme, name: n })); ui.theme.name = n; applyTheme(); render(); toast(`Saved theme "${n}"`); } },
  thdel: async () => { const n = ui.theme.name; if (ui.saved[n]) { if (await askConfirm(`Delete saved theme "${n}"?`)) { delete ui.saved[n]; saveUi(); render(); } } else toast("The current theme is not a saved theme"); },
  thexp: async () => { await navigator.clipboard.writeText(JSON.stringify(ui.theme)); toast("Theme copied to clipboard"); },
  thimp: async () => { const t = await askText("Paste theme JSON"); if (!t) return; try { const j = JSON.parse(t); ui.theme = { ...newTheme(), ...j, vars: { ...PRESETS.Graphite, ...(j.vars || {}) } }; applyTheme(); render(); } catch (_) { toast("That is not a valid theme"); } },
  bgpick: async () => { const p = await pickImage(); if (p) { ui.theme.bgImage = p; applyTheme(); render(); } },
  bgclr: () => { ui.theme.bgImage = null; applyTheme(); render(); },
  fontpick: async () => { const p = await dialogApi.open({ filters: [{ name: "Fonts", extensions: ["ttf", "otf", "woff", "woff2"] }] }); if (p) { ui.theme.fontFile = p; resetFontCache(); applyTheme(); render(); } },
  fontclr: () => { ui.theme.fontFile = null; resetFontCache(); applyTheme(); render(); },
};

const changes = {
  asklink: (d, el) => { ui.askLink = el.checked; saveUi(); },
  sflag: async (d, el) => {
    S.set[d.k] = el.checked; await saveSet();
    if (d.k === "auto_detect" && el.checked && !S.set.game_root) acts.sdetect();
    if (d.k === "debug") toast(el.checked ? "Verbose logging on" : "Verbose logging off");
  },
  sfield: async (d, el) => { S.set[d.k] = el.value.trim(); await saveSet(); if (d.k === "launch_options") render(); },
  font: (d, el) => { ui.theme.font = el.value; applyTheme(); render(); },
  fontname: (d, el) => { ui.theme.font = el.value.trim() || "Segoe UI"; applyTheme(); },
  hex: (d, el) => {
    let v = el.value.trim().replace("#", "");
    if (/^[0-9a-f]{3}$/i.test(v)) v = v.split("").map((c) => c + c).join("");
    if (!/^[0-9a-f]{6}$/i.test(v)) { toast("Enter a HEX colour such as #1a2b3c"); el.value = ui.theme.vars[d.k]; return; }
    ui.theme.vars[d.k] = "#" + v.toLowerCase(); applyTheme(); render();
  },
  headfont: (d, el) => { ui.theme.headFont = el.value; applyTheme(); },
  mono: (d, el) => { ui.theme.mono = el.value; applyTheme(); },
  thumbr: (d, el) => { ui.theme.thumbr = el.value; applyTheme(); },
  anim: (d, el) => { ui.theme.anim = el.checked; applyTheme(); },
};

const inputs = {
  txt: (d, el) => { S.txt[d.kind] = el.value; const p = $("#hl-" + d.kind); if (p) { p.innerHTML = hlCode(el.value, d.kind); p.scrollTop = el.scrollTop; p.scrollLeft = el.scrollLeft; } },
  var: (d, el) => {
    const spec = VARS.flatMap(([, i]) => i).find((x) => x[0] === d.k);
    ui.theme.vars[d.k] = spec[2] === "range" ? Number(el.value) : el.value;
    if (el.type === "range") el.style.setProperty("--p", ((el.value - el.min) / (el.max - el.min)) * 100 + "%");
    const hx = $("#h" + d.k.slice(2)); if (hx) hx.value = el.value;
    const lab = $("#v" + d.k.slice(2)); if (lab) lab.textContent = ui.theme.vars[d.k] + (spec[5] || "");
    applyTheme();
  },
};

// editor behaviour (Tab key, Ctrl+S, scroll sync), highlighted section while scrolling, live log refresh
function initEditors() {
  const C = $("#content");
  C.addEventListener("keydown", (ev) => {
    if (ev.key === "Tab" && ev.target.classList && ev.target.classList.contains("editor")) {
      ev.preventDefault();
      const t = ev.target, a = t.selectionStart;
      t.value = t.value.slice(0, a) + "\t" + t.value.slice(t.selectionEnd);
      t.selectionStart = t.selectionEnd = a + 1;
      inputs.txt({ kind: t.dataset.kind }, t);
    }
    if (ev.ctrlKey && ev.key.toLowerCase() === "s" && ev.target.id && ev.target.id.startsWith("ed-")) { ev.preventDefault(); acts.tsave({ kind: ev.target.dataset.kind }); }
  });
  C.addEventListener("scroll", (ev) => {
    const t = ev.target;
    if (t && t.classList && t.classList.contains("editor")) { const p = $("#hl-" + t.dataset.kind); if (p) { p.scrollTop = t.scrollTop; p.scrollLeft = t.scrollLeft; } return; }
    if (S.tab !== "settings") return;
    let cur = SUBS[0][0];
    for (const [k] of SUBS) { const el = $("#sec-" + k); if (el && el.getBoundingClientRect().top < 160) cur = k; }
    if (cur !== S.sub) { S.sub = cur; document.querySelectorAll(".subnav button").forEach((b) => b.classList.toggle("on", b.dataset.v === cur)); }
  }, true);
  setInterval(async () => {
    if (S.tab === "settings") { S.logs = await invoke("get_logs"); const b = $("#logbox"); if (b) { const atEnd = b.scrollTop + b.clientHeight >= b.scrollHeight - 30; b.innerHTML = logHtml(); if (atEnd) b.scrollTop = b.scrollHeight; } }
  }, 2000);
}

export default {
  id: "settings",
  tab: { label: "Settings", icon: "settings", onEnter: loadSettingsData },
  view: settingsView,
  init: initEditors,
  acts, changes, inputs,
};
