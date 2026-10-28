// Settings page: one scrolling page made of sections. To add a section: add it to SUBS and to secBody().
import { S, ui } from "../../core/state.js";
import { esc } from "../../core/dom.js";
import { ic, ib } from "../../core/icons.js";
import { pageHead } from "../../core/components.js";
import { swi, toggles, editorCard } from "./editors.js";
import { appearanceView } from "./appearance.js";
import { appUpdateBlock } from "../appupdate/view.js";

export const SUBS = [["game", "Game folder"], ["gameinfo", "gameinfo.gi"], ["launch", "Launch options"], ["autoexec", "Autoexec.cfg"], ["cursor", "Cursor"], ["downloads", "Downloads"], ["appearance", "Appearance"], ["updates", "Updates"], ["logs", "Logs"], ["debug", "Debug"]];

export const logHtml = () => S.logs.map((l) => `<div class="${l.includes(" ERROR ") ? "ERROR" : l.includes(" DEBUG ") ? "DEBUG" : ""}">${esc(l)}</div>`).join("") || "<span class='sub'>No log entries yet.</span>";
export function secBody(k) {
  const s = S.set;
  switch (k) {
    case "game": return `<div class="card"><p>${s.game_root ? esc(s.game_root) : "Not set"}</p>
      <div class="form"><span>Auto-detect game folder</span>${swi("auto_detect", s.auto_detect)}</div>
      <div class="form"><button data-act="sdetect">${ic("search")} Detect now</button><button data-act="spick">${ic("folder")} Choose folder</button>${ib("dopen", "ext", "Open folder", 'data-k="game"')}</div>
      <p style="margin:0">Default: C:\\Program Files (x86)\\Steam\\steamapps\\common\\Deadlock</p></div>`;
    case "gameinfo": return `<div class="card"><p>Status: <b>${S.patched ? "patched (mods enabled)" : "vanilla"}</b></p>
      <div class="form"><span>Auto-patch before launching with mods</span>${swi("gameinfo_auto", s.gameinfo_auto)}</div>
      <div class="form"><button class="primary" data-act="gpatch" data-force="0">${ic("check")} Patch now</button><button data-act="gpatch" data-force="1">${ic("refresh")} Force re-patch</button><button data-act="grestore">${ic("undo")} Restore vanilla</button></div>
      <p style="margin:0">Restore removes only the line Yoshirian added. A backup is kept as gameinfo.gi.yoshirian.bak.</p></div>
      <div class="card"><h3>Editor</h3>${editorCard("gameinfo", "Edits the real file. A backup (gameinfo.gi.yoshirian.edit.bak) is written on every save.")}
      <div class="form"><input type="text" data-ch="sfield" data-k="external_editor" value="${esc(s.external_editor)}" placeholder="External editor .exe (empty = Notepad)">${ib("editorpick", "folder", "Browse for editor")}</div></div>`;
    case "launch": return `<div class="card"><p>Passed to Deadlock through Steam by both launch buttons. Click a toggle to switch it on or off, or add your own.</p>
      ${toggles("launch", "New launch option, e.g. -dx11 or +fps_max 144")}
      <div class="form"><input type="text" data-ch="sfield" data-k="launch_options" value="${esc(s.launch_options)}" placeholder="Current options (editable)"></div></div>`;
    case "autoexec": return `<div class="card"><p>game/citadel/cfg/autoexec.cfg — toggles add or remove a line and save immediately.</p>
      ${toggles("auto", "New command line, e.g. fps_max 144")}${editorCard("autoexec", "Full file (created on first save if it does not exist).")}</div>`;
    case "cursor": return `<div class="card"><div class="form"><span>Create presets automatically when adding cursors</span>${swi("auto_cursor_preset", s.auto_cursor_preset)}</div>
      <p style="margin:0">A cursor pack with one set gets a preset named after the pack. A pack with several sets (folders such as Player 1, Player 2) gets one preset per folder.</p></div>`;
    case "downloads": return `<div class="card"><p>Open links from GameBanana in this app. Put <code>yoshirian://</code> in front of a GameBanana link:</p>
      <div class="kv" style="margin-bottom:14px"><span>Open the mod page</span><span>yoshirian://https://gamebanana.com/mods/599927</span><span>Install a file</span><span>yoshirian://https://gamebanana.com/dl/1854979</span><span>Install a file</span><span>yoshirian://https://gamebanana.com/mods/download/599927#FileInfo_1854979</span></div>
      <div class="form"><span>Ask before 1-click installs</span><label class="sw"><input type="checkbox" data-ch="asklink" ${ui.askLink ? "checked" : ""}><i></i></label></div>
      <p style="margin:0">Only GameBanana links are accepted. Installed mods always start disabled.</p></div>`;
    case "appearance": return appearanceView();
    case "updates": return `<div class="card"><div class="form"><span>Installed version</span><b>v${esc(S.version)}</b></div>
      ${appUpdateBlock()}
      <div class="fld">Update source — GitHub repository<div class="form" style="margin:0"><input type="text" data-ch="updrepo" value="${esc(s.update_repo)}" placeholder="${esc(S.defaultRepo)}"><button data-act="updreset" title="Back to ${esc(S.defaultRepo)}">Reset to default</button></div><small class="sub">Default: ${esc(S.defaultRepo)}. You can type owner/repo or paste a GitHub link.</small></div>
      <div class="form"><span>Check for app updates when the program starts</span>${swi("auto_update_check", s.auto_update_check)}</div>
      <hr style="border:0;border-top:1px solid var(--border);margin:14px 0">
      <div class="form"><span>Check for mod updates on startup (notify only)</span>${swi("auto_mod_update_check", s.auto_mod_update_check)}</div>
      <div class="form"><button data-act="chkupd">${ic("refresh")} Check mod updates now</button></div></div>`;
    case "logs": return `<div class="form">${ib("lrefresh", "refresh", "Refresh")}${ib("lclear", "trash", "Clear", "", "danger")}${ib("dopen", "folder", "Open log folder", 'data-k="logs"')}<small class="sub">Auto-refreshes while visible.</small></div><div class="logbox" id="logbox">${logHtml()}</div>`;
    case "debug": {
      const i = S.info || {};
      return `<div class="card"><div class="form"><span>Verbose logging</span>${swi("debug", s.debug)}</div>
      <div class="kv"><span>Version</span><span>${esc(i.version)}</span><span>Game folder</span><span>${esc(i.root || "—")}</span><span>Addons folder</span><span>${esc(i.addons || "—")}</span><span>Metadata file</span><span>${esc(i.meta_file || "—")}</span><span>Settings file</span><span>${esc(i.settings_file)}</span><span>Log folder</span><span>${esc(i.log_dir)}</span></div></div>
      <div class="form"><button data-act="dopen" data-k="addons">${ic("folder")} Addons folder</button><button data-act="dopen" data-k="data">${ic("folder")} App data</button><button data-act="dreload">${ic("refresh")} Reload mods</button><button data-act="dcopy">${ic("copy")} Copy diagnostics</button><button class="danger" data-act="dreset">${ic("undo")} Reset appearance</button></div>`;
    }
  }
}
export const settingsInner = () => `<div class="split"><div class="subnav">${SUBS.map(([k, l]) => `<button data-act="ssub" data-v="${k}" class="${S.sub === k ? "on" : ""}">${l}</button>`).join("")}</div>
  <div>${SUBS.map(([k, l]) => `<section class="sec" id="sec-${k}"><h2>${l}</h2>${secBody(k)}</section>`).join("")}</div></div>`;
export const settingsView = () => pageHead("Settings", "Game folder, launch options, appearance and more") + settingsInner();
