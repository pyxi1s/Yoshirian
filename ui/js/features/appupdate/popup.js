// The "Update available" popup: Later / Skip this version / Update, then the download progress.
import { invoke } from "../../core/tauri.js";
import { S } from "../../core/state.js";
import { esc, fmtSize } from "../../core/dom.js";
import { modal, toast } from "../../core/feedback.js";
import { skipUpdate } from "./check.js";

let live = null; // progress handler of the open popup (fed by the "app-update-progress" event)

/** Called with every progress event of the backend. */
export const onUpdateProgress = (p) => { if (live) live(p); };

const notesOf = (r) => esc((r.body || "").trim().slice(0, 1800) || "No release notes.");

/** `start` = skip the question and begin the download at once (the Update button in Settings). */
export async function openUpdatePopup({ start = false } = {}) {
  const u = S.appUpdate, r = u && u.latest;
  if (!r) return;
  let busy = false;

  const question = () => `<h3>${r.critical ? "Critical fix available" : "Update available"}${r.critical ? ' <span class="tag crit">Critical fix</span>' : ""}</h3>
    <p>Yoshirian <b>v${esc(r.version)}</b> is available. You have v${esc(u.current)}.</p>
    <div class="upd-notes">${notesOf(r)}</div>
    <div class="btns"><button data-v="later">Later</button><button data-v="skip" title="You will still be told about newer versions">Skip this version</button><button class="primary" data-v="update">${u.installer ? "Update" : "Open download page"}</button></div>`;

  const progress = (label, pct, cancel) => `<h3>Updating Yoshirian</h3>
    <p id="upd-label">${esc(label)}</p>
    <div class="bar ${pct < 0 ? "ind" : ""}"><i id="upd-fill" style="width:${Math.max(pct, 0)}%"></i></div>
    <div class="btns">${cancel ? '<button data-v="cancel">Cancel</button>' : ""}</div>`;

  const failed = (msg) => `<h3>The update could not be installed</h3><p style="white-space:pre-wrap">${esc(msg)}</p>
    <div class="btns"><button data-v="close">Close</button><button data-v="page">Open release page</button>${u.installer ? '<button class="primary" data-v="update">Try again</button>' : ""}</div>`;

  await modal('<div id="updpop"></div>', (d) => {
    const box = d.querySelector("#updpop");
    const show = (html) => { box.innerHTML = html; };
    const openPage = () => invoke("open_link", { url: r.url }).catch(() => {});

    const begin = async () => {
      if (!u.installer) { openPage(); d.close("page"); return; }
      busy = true; show(progress("Starting…", -1, true));
      try { await invoke("install_app_update"); } catch (er) { busy = false; show(failed(er)); }
    };

    live = (p) => {
      if (p.state === "downloading") {
        const pct = p.total > 0 ? Math.min(100, Math.floor((p.received / p.total) * 100)) : -1;
        const label = `Downloading… ${fmtSize(p.received)}${p.total ? " / " + fmtSize(p.total) : ""}${pct >= 0 ? " · " + pct + "%" : ""}`;
        if (!d.querySelector("#upd-fill")) show(progress(label, pct, true));
        else { d.querySelector("#upd-label").textContent = label; d.querySelector("#upd-fill").style.width = Math.max(pct, 0) + "%"; d.querySelector(".bar").classList.toggle("ind", pct < 0); }
      } else if (p.state === "verifying") show(progress("Checking the download…", 100, false));
      else if (p.state === "launching") show(progress("Starting the installer — Yoshirian will close now.", 100, false));
      else if (p.state === "cancelled") { busy = false; show(question()); }
      else if (p.state === "error") { busy = false; show(failed(p.error || "Unknown error")); }
    };

    d.addEventListener("cancel", (ev) => { if (busy) ev.preventDefault(); }); // Esc must not hide a running download
    d.addEventListener("click", async (ev) => {
      const b = ev.target.closest("[data-v]");
      if (!b) return;
      const v = b.dataset.v;
      if (v === "later" || v === "close") d.close(v);
      else if (v === "skip") { await skipUpdate(); toast(`Version ${r.version} skipped — you will be told when a newer one is released`); d.close("skip"); }
      else if (v === "update") begin();
      else if (v === "cancel") invoke("cancel_app_update");
      else if (v === "page") openPage();
    });
    if (start) begin(); else show(question());
  });
  live = null;
}
