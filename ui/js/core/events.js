// One delegated listener per event type on #content; handlers come from the feature registry.
import { $ } from "./dom.js";
import { ACTS, CHG, INP } from "./registry.js";

export function bindContentEvents() {
  const C = $("#content");
  C.addEventListener("click", async (ev) => {
    const el = ev.target.closest("[data-act]");
    if (el && el.tagName === "A") ev.preventDefault();
    if (el && ACTS[el.dataset.act]) { try { await ACTS[el.dataset.act](el.dataset, el, ev); } catch (_) {} }
  });
  C.addEventListener("change", (ev) => { const el = ev.target.closest("[data-ch]"); if (el && CHG[el.dataset.ch]) CHG[el.dataset.ch](el.dataset, el); });
  C.addEventListener("input", (ev) => { const el = ev.target.closest("[data-in]"); if (el && INP[el.dataset.in]) INP[el.dataset.in](el.dataset, el); });
  // <input data-enter="actionName"> runs that action when Enter is pressed
  C.addEventListener("keydown", (ev) => { if (ev.key === "Enter" && ev.target.dataset.enter) ACTS[ev.target.dataset.enter](ev.target.dataset); });
}
