// Settings → Appearance (theme presets, colours, fonts).
import { ui } from "../../core/state.js";
import { esc } from "../../core/dom.js";
import { ic, ib } from "../../core/icons.js";
import { VARS, FONTS, HEAD_FONTS, PRESETS } from "../../core/theme-data.js";

export const SHADES = [["--bg", "Background"], ["--side", "Sidebar"], ["--panel", "Panel"], ["--card", "Card"], ["--border", "Border"], ["--text", "Text"], ["--sub", "Muted text"], ["--accent", "Accent"], ["--accent2", "Accent 2"], ["--on-accent", "Text on accent"], ["--danger", "Danger"], ["--danger-bg", "Delete button"], ["--ok", "Success"], ["--warn", "Warning"]];
export function themeCards() {
  const all = [...Object.entries(PRESETS).map(([n, v]) => [n, v, "p:" + n]), ...Object.entries(ui.saved).map(([n, t]) => [n, t.vars, "s:" + n])];
  return `<div class="themes">${all.map(([n, v, key]) => `<button class="tcard ${ui.theme.name === n ? "on" : ""}" data-act="theme" data-k="${esc(key)}">
    <div class="tprev" style="background:${v["--bg"]}"><div class="ts" style="background:${v["--side"]}"><i style="background:${v["--accent"]}"></i><i style="background:${v["--panel"]}"></i><i style="background:${v["--panel"]}"></i></div>
      <div class="tm"><b style="color:${v["--text"]}">Aa</b><small style="color:${v["--sub"]}">Muted text</small><div class="tc" style="background:${v["--card"]};border-color:${v["--border"]}"><i style="background:${v["--accent"]}"></i><i style="background:${v["--accent2"]}"></i></div></div></div>
    <div class="swr">${SHADES.map(([k, l]) => `<i style="background:${v[k]}" title="${l} ${v[k]}"></i>`).join("")}</div>
    <span>${key[0] === "s" ? "★ " : ""}${esc(n)}</span></button>`).join("")}</div>`;
}
export function appearanceView() {
  const t = ui.theme;
  const ctl = ([k, label, type, min, max, unit, step]) => type === "color"
    ? `<label>${label}<span class="cw"><input type="text" class="hex" id="h${k.slice(2)}" data-ch="hex" data-k="${k}" value="${t.vars[k]}" maxlength="7" spellcheck="false"><input type="color" data-in="var" data-k="${k}" value="${t.vars[k]}"></span></label>`
    : `<label>${label}<span><input type="range" data-in="var" data-k="${k}" min="${min}" max="${max}" step="${step}" value="${t.vars[k]}" style="--p:${((t.vars[k] - min) / (max - min)) * 100}%"> <small class="sub" id="v${k.slice(2)}">${t.vars[k]}${unit}</small></span></label>`;
  return `<div class="card"><h3>Theme</h3><p>Look of the app: colours, corners, spacing and fonts. Applies immediately.</p>${themeCards()}
    <div class="form" style="margin-top:12px"><button data-act="thsave">${ic("save")} Save as…</button><button class="danger" data-act="thdel">${ic("trash")} Delete saved</button><button data-act="thexp">${ic("copy")} Export</button><button data-act="thimp">${ic("ext")} Import</button></div></div>
  ${VARS.map(([g, items]) => `<div class="card"><h3>${g}</h3><div class="vars">${items.map(ctl).join("")}</div>${g === "Background image" ? `<div class="form" style="margin-top:12px"><button data-act="bgpick">${ic("image")} Choose image</button>${ib("bgclr", "trash", "Remove image", "", "danger")}</div>` : ""}</div>`).join("")}
  <div class="card"><h3>Font</h3><div class="vars"><label>Interface font<select data-ch="font">${FONTS.map((f) => `<option ${t.font === f ? "selected" : ""}>${f}</option>`).join("")}</select></label>
    <label>Heading font<select data-ch="headfont">${HEAD_FONTS.map((f) => `<option ${t.headFont === f ? "selected" : ""}>${f}</option>`).join("")}</select></label>
    <label>Custom font name<input type="text" data-ch="fontname" value="${esc(t.font)}"></label>
    <label>Code font<select data-ch="mono">${["Consolas", "Cascadia Mono", "Courier New", "Lucida Console"].map((f) => `<option ${t.mono === f ? "selected" : ""}>${f}</option>`).join("")}</select></label></div>
    <div class="form" style="margin-top:12px"><button data-act="fontpick">${ic("plus")} Font file (.ttf / .otf / .woff2)</button>${ib("fontclr", "trash", "Remove font file", "", "danger")}<small class="sub">${t.fontFile ? esc(t.fontFile.split(/[\\/]/).pop()) : "No custom file"}</small></div></div>
  <div class="card"><h3>Other</h3><div class="vars"><label>Thumbnail ratio<select data-ch="thumbr">${["8/7", "16/9", "4/3", "1/1", "3/4", "21/9"].map((r) => `<option ${t.thumbr === r ? "selected" : ""}>${r}</option>`).join("")}</select></label>
    <label>Animations<label class="sw"><input type="checkbox" data-ch="anim" ${t.anim ? "checked" : ""}><i></i></label></label></div></div>`;
}
