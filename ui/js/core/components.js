// Small HTML fragments reused by several features.
import { ui } from "./state.js";
import { ic } from "./icons.js";

export const viewSeg = (k) => `<div class="seg"><button title="List" data-act="view" data-k="${k}" data-v="list" class="${ui[k] === "list" ? "on" : ""}">${ic("list")}</button><button title="Cards" data-act="view" data-k="${k}" data-v="card" class="${ui[k] === "card" ? "on" : ""}">${ic("grid")}</button></div>`;
export const pageHead = (t, sub) => `<div class="head"><div><h1>${t}</h1><div class="sub">${sub}</div></div></div>`;
