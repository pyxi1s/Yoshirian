// DOM + formatting helpers (pure functions, no app state).
import { convertFileSrc } from "./tauri.js";

export const $ = (s) => document.querySelector(s);
export const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
export const fmtSize = (n) => (n > 1048576 ? (n / 1048576).toFixed(1) + " MB" : Math.max(1, Math.round(n / 1024)) + " KB");
export const fmtNum = (n) => Number(n || 0).toLocaleString("en-US");
export const fmtDate = (t) => (t ? new Date(t * 1000).toLocaleDateString("en-US", { year: "numeric", month: "short", day: "numeric" }) : "—");
export const bg = (u, extra = "") => (u || extra ? `style='${extra}${u ? `;background-image:url("${String(u).replace(/'/g, "%27")}")` : ""}'` : "");
export const local = (p) => (p ? convertFileSrc(p) : "");
export const linkify = (t) => String(t).split(/(https?:\/\/[^\s<>"']*[^\s<>"'.,;:!?)\]])/g).map((p, i) => (i % 2 ? `<a href="#" data-act="openlink" data-url="${esc(p)}">${esc(p)}</a>` : esc(p))).join("");
