// Settings -> Updates: status of the app update, with Update / Skip / Check buttons.
import { S } from "../../core/state.js";
import { esc } from "../../core/dom.js";
import { ic } from "../../core/icons.js";

export function appUpdateBlock() {
  const u = S.appUpdate, r = u && u.latest;
  let status;
  if (S.appUpdChecking) status = '<span class="sub">Checking for updates…</span>';
  else if (S.appUpdError) status = `<span style="color:var(--danger)">${esc(S.appUpdError)}</span>`;
  else if (!u) status = '<span class="sub">Not checked yet.</span>';
  else if (!r) status = `<span style="color:var(--ok)">You are on the latest version.</span> <span class="sub">(${esc(u.repo)})</span>`;
  else status = `<b>Version ${esc(r.version)} is available</b>${r.critical ? ' <span class="tag crit">Critical fix</span>' : ""}${u.skipped ? ' <span class="sub">— you skipped this version</span>' : ""}`;

  const notes = r && !S.appUpdChecking ? `<div class="upd-notes" style="margin:8px 0">${esc((r.body || "").trim().slice(0, 1800) || "No release notes.")}</div>` : "";
  const canAct = r && !S.appUpdChecking;
  const buttons = [
    canAct ? `<button class="primary" data-act="updnow">${ic("downloads")} ${u.skipped ? "Update anyway" : u.installer ? "Update" : "Open download page"}</button>` : "",
    canAct ? (u.skipped ? '<button data-act="updunskip">Undo skip</button>' : '<button data-act="updskip" title="You will still be told about newer versions">Skip this version</button>') : "",
    `<button data-act="updcheck" ${S.appUpdChecking ? "disabled" : ""}>${ic("refresh")} Check now</button>`,
  ].join("");
  return `<div class="form" style="align-items:flex-start;flex-direction:column;gap:6px"><div>${status}</div>${notes}<div class="form" style="margin:0">${buttons}</div></div>`;
}
