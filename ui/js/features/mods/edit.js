// "Edit mod" dialog.
import { invoke } from "../../core/tauri.js";
import { S } from "../../core/state.js";
import { esc, fmtSize, bg, local } from "../../core/dom.js";
import { ic } from "../../core/icons.js";
import { modal, toast, pickImage } from "../../core/feedback.js";
import { refresh } from "./data.js";

export async function openEdit(id) {
  const m = S.mods.find((x) => x.yoshirian_id === id);
  if (!m) return;
  let img = null, clear = false;
  const res = await modal(`<h3>Edit mod</h3>
    <div class="fld">Image<div style="display:flex;gap:12px;align-items:center"><div class="thumb" id="eprev" ${bg(local(m.image_path), "width:160px")}></div>
      <button type="button" id="epick">${ic("image")} Choose</button><button type="button" id="eclr" class="ib danger" title="Remove image">${ic("trash")}</button></div></div>
    <div class="fld">Name (required)<input type="text" id="ename" value="${esc(m.name)}"></div>
    <div class="fld">Description<textarea id="edesc" rows="4">${esc(m.description)}</textarea></div>
    <div class="fld">Requires (mod names, comma separated)<input type="text" id="ereq" value="${esc((m.requires || []).join(", "))}"></div>
    <div class="fld">File<span>${esc(m.current_file)} · ${fmtSize(m.size)}${m.gb_id ? ` · GameBanana #${m.gb_id}` : ""}</span></div>
    <form method="dialog" class="btns"><button value="cancel" title="Cancel">${ic("x")}</button><button type="button" class="primary" id="esave">${ic("save")} Save</button></form>`, (d) => {
    const prev = d.querySelector("#eprev");
    d.querySelector("#epick").onclick = async () => { const p = await pickImage(); if (p) { img = p; clear = false; prev.style.backgroundImage = `url("${local(p)}")`; } };
    d.querySelector("#eclr").onclick = () => { img = null; clear = true; prev.style.backgroundImage = "none"; };
    d.querySelector("#esave").onclick = async () => {
      const name = d.querySelector("#ename").value.trim();
      if (!name) return toast("Mod name is required");
      try {
        await invoke("edit_mod", { args: { id, name, description: d.querySelector("#edesc").value, requires: d.querySelector("#ereq").value.split(",").map((x) => x.trim()).filter(Boolean), image: img, clear_image: clear } });
        d.close("save");
      } catch (er) { toast(er); }
    };
  });
  if (res === "save") refresh();
}
