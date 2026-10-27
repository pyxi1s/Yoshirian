// Profiles dialog (opened from the titlebar pill).
import { invoke } from "../../core/tauri.js";
import { S, ui, saveUi } from "../../core/state.js";
import { esc } from "../../core/dom.js";
import { ic } from "../../core/icons.js";
import { modal, toast, askConfirm } from "../../core/feedback.js";
import { render } from "../../core/render.js";

export async function openProfiles() {
  const pb = (k, icon, title, n, cls = "") => `<button class="ib sm ${cls}" title="${title}" data-p="${k}" data-n="${esc(n)}">${ic(icon)}</button>`;
  const html = () => `<h3>Profiles</h3>${S.profiles.length ? `<div class="list" style="margin-bottom:14px">${S.profiles.map((p) => `<div class="row"><span class="radio ${ui.activeProfile === p.name ? "on" : ""}" title="${ui.activeProfile === p.name ? "Active profile" : "Not active"}" data-p="apply" data-n="${esc(p.name)}"></span><div class="nm"><b>${esc(p.name)}</b><small>${p.enabled.length} mods enabled</small></div>
      ${pb("apply", "play", "Apply this profile", p.name, "primary")}${pb("update", "refresh", "Update with the mods enabled now", p.name)}${pb("del", "trash", "Delete", p.name, "danger")}</div>`).join("")}</div>` : `<p class="sub">No profiles yet. Enable the mods you want, then save them as a profile.</p>`}
    <div class="form"><input type="text" id="pnew" placeholder="New profile name"><button class="primary" data-p="add">${ic("plus")} Add current setup</button></div>
    <div class="btns"><button data-p="close" title="Close">${ic("x")}</button></div>`;
  await modal(html(), (d) => {
    d.onclick = async (ev) => {
      const b = ev.target.closest("[data-p]");
      if (!b) return;
      const n = b.dataset.n, k = b.dataset.p;
      try {
        if (k === "close") return d.close();
        if (k === "apply") { await invoke("apply_profile_cmd", { name: n }); ui.activeProfile = n; saveUi(); toast(`Applied "${n}"`); }
        if (k === "update") { if (!(await askConfirm(`Overwrite "${n}" with the mods enabled right now?`))) return; await invoke("save_profile_cmd", { name: n }); toast(`Updated "${n}"`); }
        if (k === "del") { if (!(await askConfirm(`Delete profile "${n}"?`))) return; await invoke("delete_profile", { name: n }); if (ui.activeProfile === n) { ui.activeProfile = null; saveUi(); } }
        if (k === "add") { const v = d.querySelector("#pnew").value.trim(); if (!v) return toast("Enter a profile name"); await invoke("save_profile_cmd", { name: v }); toast(`Saved "${v}"`); }
      } catch (er) { return toast(er); }
      S.mods = await invoke("list_mods"); S.profiles = await invoke("list_profiles");
      d.innerHTML = html();
      render();
    };
  });
}
