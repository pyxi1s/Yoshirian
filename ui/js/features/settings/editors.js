// Pieces shared by the settings sections: switch, toggle chips and the text editor card.
import { S, ui } from "../../core/state.js";
import { esc } from "../../core/dom.js";
import { ib } from "../../core/icons.js";
import { hlCode } from "./highlight.js";

export const swi = (k, on) => `<label class="sw"><input type="checkbox" data-ch="sflag" data-k="${k}" ${on ? "checked" : ""}><i></i></label>`;
export const isOn = (kind, t) => kind === "launch" ? (" " + (S.set.launch_options || "").trim() + " ").includes(" " + t + " ") : (S.txt.autoexec || "").split(/\r?\n/).map((l) => l.trim()).includes(t.trim());
export function toggles(kind, hint) {
  const list = kind === "launch" ? ui.lt : ui.at;
  return `<div class="chips">${list.map((t, i) => `<span class="chip ${isOn(kind, t) ? "on" : ""}" data-act="tg" data-kind="${kind}" data-i="${i}" title="Click to toggle">${esc(t)}<b data-act="tgx" data-kind="${kind}" data-i="${i}" title="Remove this toggle">×</b></span>`).join("") || '<span class="sub">No toggles yet.</span>'}</div>
  <div class="form"><input type="text" id="tgnew-${kind}" data-enter="tgadd" data-kind="${kind}" placeholder="${hint}">${ib("tgadd", "plus", "Add toggle", `data-kind="${kind}"`, "primary")}</div>`;
}
export const editorCard = (kind, note) => `<p>${note}</p><div class="ed-wrap"><pre id="hl-${kind}">${hlCode(S.txt[kind] ?? "", kind)}</pre><textarea class="editor" id="ed-${kind}" data-in="txt" data-kind="${kind}" spellcheck="false">${esc(S.txt[kind] ?? "")}</textarea></div>
  <div class="form" style="margin-top:10px">${ib("tsave", "save", "Save (Ctrl+S)", `data-kind="${kind}"`, "primary")}${ib("treload", "refresh", "Reload from disk", `data-kind="${kind}"`)}${ib("text", "ext", "Open in external editor", `data-kind="${kind}"`)}</div>`;

/** The list of toggle chips for "launch" (launch options) or "auto" (autoexec.cfg). */
export const setList = (kind) => (kind === "launch" ? ui.lt : ui.at);
