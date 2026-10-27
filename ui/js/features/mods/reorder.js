// Drag to reorder mods (pointer based: HTML5 drag is disabled by the native file-drop handler).
import { invoke } from "../../core/tauri.js";
import { S } from "../../core/state.js";
import { $ } from "../../core/dom.js";
import { toast } from "../../core/feedback.js";
import { refresh } from "./data.js";

export function initReorder() {
  const C = $("#content");
  let drag = null;
  const clearOver = () => C.querySelectorAll(".over,.dragging").forEach((x) => x.classList.remove("over", "dragging"));
  C.addEventListener("pointerdown", (ev) => {
    const g = ev.target.closest(".grip");
    if (!g || ev.button !== 0) return;
    const item = g.closest("[data-id]");
    if (!item) return;
    drag = { id: item.dataset.id, over: null };
    item.classList.add("dragging");
    g.setPointerCapture(ev.pointerId);
    ev.preventDefault();
  });
  C.addEventListener("pointermove", (ev) => {
    if (!drag) return;
    const t = document.elementFromPoint(ev.clientX, ev.clientY);
    const el = t && t.closest("[data-id]");
    C.querySelectorAll(".over").forEach((x) => x.classList.remove("over"));
    drag.over = null;
    if (el && el.dataset.id !== drag.id && el.closest("#content")) { el.classList.add("over"); drag.over = el.dataset.id; }
  });
  C.addEventListener("pointerup", async () => {
    if (!drag) return;
    const d = drag; drag = null; clearOver();
    if (!d.over) return;
    const ids = S.mods.map((m) => m.yoshirian_id), from = ids.indexOf(d.id), to = ids.indexOf(d.over);
    ids.splice(from, 1); ids.splice(to, 0, d.id); // take the target's position
    try { await invoke("reorder_mods", { ids }); } catch (er) { toast(er); }
    refresh();
  });
  C.addEventListener("pointercancel", () => { drag = null; clearOver(); });
}
