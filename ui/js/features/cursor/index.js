// Feature: Cursor tab — replace the in-game cursors, presets, cursor packs.
import "./state.js";
import { S } from "../../core/state.js";
import { $ } from "../../core/dom.js";
import { toast, askConfirm, askText } from "../../core/feedback.js";
import { render } from "../../core/render.js";
import { cursorView } from "./view.js";
import { loadCursor, cursorCall } from "./data.js";
import { openCursorAdd } from "./add.js";

export default {
  id: "cursor",
  tab: { label: "Cursor", icon: "cursor", onEnter: loadCursor },
  view: cursorView,
  acts: {
    ckind: (d) => { S.ck = d.v; render(); },
    curadd: () => openCursorAdd(),
    curedit: async (d) => { const m = S.cur.mods.find((x) => x.id === d.id), n = await askText("Cursor name", m.name); if (n) await cursorCall("cursor_rename", { id: d.id, name: n }); },
    curdel: async (d) => { const m = S.cur.mods.find((x) => x.id === d.id); if (await askConfirm(`Delete cursor "${m.name}"?`)) await cursorCall("cursor_delete", { id: d.id }); },
    cuse: (d) => cursorCall("cursor_activate", { kind: d.k, id: S.cur.active[d.k] === d.id ? null : d.id }),
    curreset: async () => { if (await askConfirm("Restore all four original cursors?")) { await cursorCall("cursor_restore_all"); toast("Original cursors restored"); } },
    curreapply: async () => { await cursorCall("cursor_reapply"); toast("Cursors re-applied"); },
    cpadd: async () => { const el = $("#cpname"), n = (el && el.value || "").trim(); if (!n) return toast("Enter a preset name"); await cursorCall("cursor_profile", { op: "save", name: n }); toast(`Saved preset "${n}"`); },
    cpapply: async (d) => { await cursorCall("cursor_profile", { op: "apply", name: d.n }); toast(`Applied "${d.n}"`); },
    cpupdate: async (d) => { if (await askConfirm(`Overwrite "${d.n}" with the cursors in use now?`)) await cursorCall("cursor_profile", { op: "save", name: d.n }); },
    cpdel: async (d) => { if (await askConfirm(`Delete preset "${d.n}"?`)) await cursorCall("cursor_profile", { op: "delete", name: d.n }); },
  },
  changes: {
    cuse: (d, el) => cursorCall("cursor_activate", { kind: d.k, id: el.value || null }),
  },
  inputs: {
    cq: (d, el) => { S.cq = el.value; render(); },
  },
};
