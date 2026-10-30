// The Downloads tab: GameBanana search / detail pages, queue and local upload.
import { S, ui, activeDls } from "../../core/state.js";
import { esc, fmtSize, fmtNum, fmtDate, bg, local } from "../../core/dom.js";
import { richHtml, plainText } from "../../core/richtext.js";
import { ic, ib } from "../../core/icons.js";
import { viewSeg, pageHead } from "../../core/components.js";
import { waveHtml } from "../../services/audio.js";
import { soundPanel } from "../mods/sounds.js";
import { updateBanner } from "../mods/detail.js";

let INST = new Set(); // "<model>:<gb id>" of installed mods, rebuilt on every render
export const stat = (i, n) => `<span style="display:inline-flex;gap:4px;align-items:center">${ic(i)}${fmtNum(n)}</span>`;
export function resultCard(m) {
  const stats = `<div class="stats">${stat("heart", m.likes)}${stat("eye", m.views)}</div>`;
  const cat = [m.super_category, m.category].filter(Boolean).join(" › ");
  const md = m.model || "Mod", dat = `data-id="${m.id}" data-model="${md}"`;
  const playBtn = (cls) => (md === "Sound" ? `<button class="ib ${cls}" title="Preview the sound" data-act="playsnd" data-url="${esc(m.audio || "")}" ${dat}>${ic("play")}</button>` : "");
  const inst = INST.has(`${md}:${m.id}`) ? '<span class="tag on" style="margin:0">Installed</span>' : ib("gbinst", "downloads", "Install", dat, "sm primary");
  if (ui.vd === "list")
    return `<div class="row click" data-act="gbopen" ${dat}><div class="nm"><b>${esc(m.name)}${md === "Sound" ? ' <span class="tag">Sound</span>' : ""}</b><small>${esc(m.author)}${cat ? " · " + esc(cat) : ""} · ${fmtDate(m.added)}</small></div>${stats}${playBtn("sm")}${inst}</div>`;
  return `<div class="cardm click" data-act="gbopen" ${dat}><div class="img" ${bg(m.thumb)}>${playBtn("playov")}</div><div class="body"><b>${esc(m.name)}</b><div class="desc">${md === "Sound" ? "Sound · " : ""}${esc(m.author)}${cat ? " · " + esc(cat) : ""}</div></div><div class="foot">${stats}<span class="grow"></span>${inst}</div></div>`;
}
/** The uploader's short note under a file name, e.g. "All playable files combined". */
export const fileNote = (f) => (f.description ? `<div class="file-note">${esc(plainText(f.description))}</div>` : "");

const isOpen = (k, dflt = false) => (S.gbOpen[k] === undefined ? dflt : S.gbOpen[k]);
const toggler = (k, open) => `data-act="gbtoggle" data-k="${k}" aria-expanded="${open}"`;

/** All files of the page with their notes. Collapsed by default (the file picker shows the same notes). */
const filesCard = (m) => {
  if (!m.files.length) return "";
  const open = isOpen("files");
  const rows = m.files.map((f) => `<div class="row"><div class="nm"><b>${esc(f.name)}</b>${fileNote(f)}<small>${fmtSize(f.size)}${f.added ? " · " + fmtDate(f.added) : ""}</small></div></div>`).join("");
  return `<div class="card fold ${open ? "open" : ""}"><div class="fold-head" ${toggler("files", open)}><h3>Files <span class="sub">(${m.files.length})</span></h3></div>${open ? `<div class="list">${rows}</div>` : ""}</div>`;
};

const CAT = { addition: "add", improvement: "imp", bugfix: "bug", overhaul: "ovh", adjustment: "adj", removal: "rem", change: "adj" };
const catClass = (c) => CAT[String(c || "").trim().toLowerCase()] || "oth";

/** The page's update log: newest first, the latest one open, every entry can be folded. */
const updatesCard = (m) => {
  if (!m.updates || !m.updates.length) return "";
  const rows = m.updates.map((u, i) => {
    const open = isOpen("u" + i, i === 0);
    const counts = {};
    for (const c of u.changes) counts[c.cat] = (counts[c.cat] || 0) + 1;
    const summary = Object.entries(counts).map(([c, n]) => `<span class="cat ${catClass(c)}">${esc(c || "Change")}${n > 1 ? ` <b>${n}</b>` : ""}</span>`).join("");
    const lines = u.changes.map((c) => `<div class="chg"><span class="cat ${catClass(c.cat)}">${esc(c.cat || "Change")}</span><span>${esc(plainText(c.text))}</span></div>`).join("");
    const text = u.text ? `<div class="rich">${richHtml(u.text)}</div>` : "";
    const files = u.files.length ? `<div class="upd-files">${u.files.map((f) => `<code>${esc(f)}</code>`).join("")}</div>` : "";
    const body = open ? `<div class="upd-body">${lines}${text}${files}</div>` : "";
    return `<div class="upd ${open ? "open" : ""}"><div class="upd-head" ${toggler("u" + i, open)}><span class="upd-name">${esc(u.name || "Update")}</span>${u.version ? `<span class="upd-ver">${esc(u.version)}</span>` : ""}<span class="grow"></span><span class="upd-date">${u.date ? fmtDate(u.date) : ""}</span>${open ? "" : `<span class="upd-sum">${summary}</span>`}</div>${body}</div>`;
  }).join("");
  return `<div class="card"><h3>Updates</h3>${rows}</div>`;
};

/** The author name opens a search for everything this author made. */
const authorLink = (m) => (m.author ? `<a href="#" data-act="gbauthor" data-id="${m.author_id || 0}" data-name="${esc(m.author)}" title="Show everything by ${esc(m.author)}">${esc(m.author)}</a>` : "—");

export function gbDetail(m) {
  const sound = (m.model || "Mod") === "Sound";
  const copies = S.mods.filter((x) => x.gb_id === m.id && (x.gb_model || "Mod") === (m.model || "Mod"));
  const installed = copies.length > 0;
  // "Update available" with Update / Skip (or the "skipped — Undo" note) for the installed copy
  const banner = installed ? updateBanner(copies.find((x) => S.updates[x.yoshirian_id]) || copies.find((x) => x.gb_skipped > x.gb_updated) || copies[0]) : "";
  const files = m.files.length ? `<div class="form"><button class="grad" data-act="gbdl">${ic("downloads")} Install${m.files.length > 1 ? ` (${m.files.length} files — choose)` : ""}</button>${installed ? `<button data-act="gbdl" data-update="1" title="Download a file and replace the installed one">${ic("refresh")} Replace installed</button>` : ""}</div>${m.files.length > 1 ? `<div class="sub" style="font-size:12px;margin:-6px 0 10px">${m.files.map((f) => esc(f.name)).join(" · ")}</div>` : ""}` : `<p class="sub">This item has no downloadable files.</p>`;
  const thumb = sound ? "" : `<div class="thumb" ${bg(m.thumb, "width:360px;max-width:100%;aspect-ratio:16/9;cursor:zoom-in")} data-act="lightbox" data-i="0"></div>`;
  return `${ib("gbback", "back", "Back")}
  ${banner}
  <div class="card" style="margin-top:14px"><div style="display:flex;gap:20px;flex-wrap:wrap">${thumb}
    <div style="flex:1;min-width:260px"><h1>${esc(m.name)} ${ib("openlink", "ext", "Open on GameBanana", `data-url="${esc(m.url)}"`, "ghost")}</h1>
      <div class="stats" style="margin-bottom:10px">${sound ? '<span class="tag">Sound</span>' : ""}${installed ? '<span class="tag on">Installed</span>' : ""}${stat("heart", m.likes)}${stat("downloads", m.downloads)}${stat("eye", m.views)}</div>
      <div class="kv" style="margin-bottom:12px"><span>Author</span><span>${authorLink(m)}</span><span>Added</span><span>${fmtDate(m.added)}</span><span>Updated</span><span>${fmtDate(m.updated)}</span><span>Type / hero</span><span>${esc([m.super_category, m.category].filter(Boolean).join(" › ") || "—")}</span><span>Page</span><span><a href="#" data-act="openlink" data-url="${esc(m.url)}">${esc(m.url)}</a></span></div>${files}
      <small class="sub">Installed mods start disabled. Enable them in the Mods tab.</small></div></div></div>
  ${filesCard(m)}
  ${m.audio ? `<div class="card"><h3>Preview</h3>${waveHtml(m.audio)}</div>` : ""}
  ${S.mods.filter((x) => x.gb_id === m.id && (x.gb_model || "Mod") === (m.model || "Mod") && x.snd > 0).map((x) => soundPanel(x.yoshirian_id, "Sounds in your installed copy")).join("")}
  ${m.images.length > 1 ? `<div class="card"><h3>Screenshots</h3><div class="gallery">${m.images.map((u, i) => `<div ${bg(u)} data-act="lightbox" data-i="${i}"></div>`).join("")}</div></div>` : ""}
  ${updatesCard(m)}
  <div class="card"><h3>Description</h3>${m.description ? `<p class="sub" style="user-select:text">${esc(plainText(m.description))}</p>` : ""}<div class="rich">${richHtml(m.text) || (m.description ? "" : '<span class="sub">No description.</span>')}</div></div>`;
}
/** Text / numbers of one queue row (shared by the full render and the in-place update). */
export function dlInfo(d) {
  const known = d.total > 0;
  const pct = d.state === "done" ? 100 : known ? Math.min(100, Math.floor((d.received / d.total) * 100)) : 0;
  const size = `${fmtSize(d.received)}${known ? " / " + fmtSize(d.total) : ""}`;
  const label = { queued: "Queued — waiting for its turn", downloading: `Downloading · ${size}`, paused: `Paused · ${size}`, installing: "Installing…",
    done: d.kind === "cursor" ? `Added ${d.installed} cursor(s) to the Cursor library` : `Installed ${d.installed} mod(s) (disabled)`,
    error: "Failed — " + (d.error || ""), cancelled: "Cancelled" }[d.state] || d.state;
  const running = d.state === "downloading" || d.state === "installing";
  return { pct, label, indeterminate: running && (!known || d.state === "installing"), pctText: known || d.state === "done" ? pct + "%" : d.state === "downloading" ? "…" : "" };
}

/** Update a queue row without re-rendering the page (a re-render would swallow clicks on its buttons). */
export function patchDlRow(d) {
  const row = document.querySelector(`[data-dl="${d.id}"]`);
  if (!row) return false;
  const i = dlInfo(d);
  row.querySelector(".dl-label").textContent = i.label;
  row.querySelector(".dl-fill").style.width = i.pct + "%";
  row.querySelector(".dl-pct").textContent = i.pctText;
  row.querySelector(".bar").classList.toggle("ind", i.indeterminate);
  return true;
}

export function myUpdatesView() {
  const seen = new Set(), items = [];
  for (const id of Object.keys(S.updates)) {
    const m = S.mods.find((x) => x.yoshirian_id === id); if (!m) continue;
    const key = `${m.gb_id}:${m.gb_model}`; if (seen.has(key)) continue; seen.add(key); items.push(m);
  }
  const head = `<div class="head" style="margin-bottom:10px"><h3 style="margin:0">Updates for your mods</h3><span class="grow"></span>${items.length > 1 ? `<button class="primary" data-act="updall">${ic("downloads")} Update all</button>` : ""}${ib("chkupd", "refresh", "Check for mod updates")}<button data-act="gblatest">Browse latest on GameBanana</button></div>`;
  if (!items.length) return head + `<div class="empty">${ic("check")}<div>${S.updChecked ? "All your mods are up to date." : "Checking for updates…"}</div><div style="margin-top:6px">Search GameBanana above to find mods and sounds.</div></div>`;
  const one = (m) => {
    const dat = `data-id="${m.gb_id}" data-model="${m.gb_model || "Mod"}"`, up = ib("modupd", "downloads", "Update", `data-id="${m.yoshirian_id}"`, "sm primary") + ib("modskip", "skip", "Skip this update (you will still be told about newer ones)", `data-id="${m.yoshirian_id}"`, "sm ghost");
    return ui.vd === "list"
      ? `<div class="row click" data-act="gbopen" ${dat}><div class="nm"><b>${esc(m.name)}</b><small>${m.author ? "By " + esc(m.author) + " · " : ""}update available</small></div>${up}</div>`
      : `<div class="cardm click" data-act="gbopen" ${dat}><div class="img ${m.image_path ? "" : "ph"}" ${bg(local(m.image_path))}>${m.image_path ? "" : "↑"}</div><div class="body"><b>${esc(m.name)}</b><div class="by">${m.author ? "By " + esc(m.author) + " · " : ""}update available</div></div><div class="foot"><span class="grow"></span>${up}</div></div>`;
  };
  return head + (ui.vd === "list" ? `<div class="list">${items.map(one).join("")}</div>` : `<div class="grid">${items.map(one).join("")}</div>`);
}
export function downloadsInner() {
  INST = new Set(S.mods.filter((x) => x.gb_id).map((x) => `${x.gb_model || "Mod"}:${x.gb_id}`));
  const subs = `<div class="tabrow"><div class="tabs">${[["queue", "Queue" + (activeDls() ? ` (${activeDls()})` : "")], ["download", "Download"], ["upload", "Local Upload"]].map(([k, l]) => `<button data-act="dsub" data-v="${k}" class="${S.dlSub === k ? "on" : ""}">${l}</button>`).join("")}</div><button data-act="openlink" data-url="https://gamebanana.com/games/20948">${ic("ext")} GameBanana store</button></div>`;
  if (S.dlSub === "queue") {
    const l = Object.values(S.dls).reverse();
    return `${subs}${l.length ? `<div class="list">${l.map((d) => {
      const i = dlInfo(d);
      const act = (a, ic_, title, cls) => ib(a, ic_, title, `data-id="${d.id}"`, "sm " + cls);
      const buttons = d.state === "downloading" || d.state === "queued" ? act("dlpause", "pause", "Pause", "ghost") + act("dlcancel", "x", "Cancel download", "del")
        : d.state === "paused" ? act("dlresume", "play", "Resume", "primary") + act("dlcancel", "x", "Cancel download", "del") : "";
      return `<div class="row dl ${d.state}" data-dl="${d.id}"><div class="nm"><b>${esc(d.name)}</b><small class="dl-label">${esc(i.label)}</small><div class="bar ${i.indeterminate ? "ind" : ""}"><i class="dl-fill" style="width:${i.pct}%"></i></div></div><span class="dl-pct">${i.pctText}</span><div class="dl-actions">${buttons}</div></div>`;
    }).join("")}</div><div class="form" style="margin-top:12px"><button data-act="qclear">${ic("trash")} Clear finished</button></div>` : `<div class="empty">${ic("downloads")}<div>The queue is empty.</div></div>`}`;
  }
  if (S.dlSub === "upload") {
    const items = S.up.map((it, i) => `<div class="card"><div style="display:flex;gap:14px"><div><div class="thumb" ${bg(local(it.image), "width:150px")}></div>
      <div style="margin-top:8px;display:flex;gap:6px">${ib("upimg", "image", "Choose image", `data-i="${i}"`, "sm")}${ib("upimgx", "x", "Clear image", `data-i="${i}"`, "sm")}</div></div>
      <div style="flex:1"><div class="fld">Name (required) — ${esc(it.source.split(/[\\/]/).pop())}<input type="text" data-in="up" data-i="${i}" data-f="name" value="${esc(it.name)}"></div>
      <div class="fld">Description (optional)<textarea rows="2" data-in="up" data-i="${i}" data-f="description">${esc(it.description)}</textarea></div></div>
      ${ib("upx", "trash", "Remove", `data-i="${i}"`, "sm danger")}</div></div>`).join("");
    return `${subs}<div class="drop-zone">Drag &amp; drop .vpk or an archive (${S.exts.map((x) => "." + x).join(", ")}) anywhere in this window<br><br><button class="primary" data-act="upick">${ic("plus")} Choose files</button></div>${items}
      ${S.up.length ? `<button class="grad" data-act="uinstall">${ic("downloads")} Install ${S.up.length} mod(s)</button> <small class="sub">They will be added disabled.</small>` : ""}`;
  }
  let body;
  if (S.loading) body = `<div class="empty">Loading…</div>`;
  else if (S.gbView) body = gbDetail(S.gbView);
  else if (S.gbList) {
    const list = S.gbList.filter((m) => S.gm === "all" || (m.model || "Mod") === S.gm);
    const ft = [["Mod", "Mods"], ["all", "All"], ["Sound", "Sounds"]]; // All in the middle
    body = `<div class="head" style="margin-bottom:10px"><h3 style="margin:0">${esc(S.gbTitle)}</h3><span class="grow"></span><div class="tabs" style="margin:0">${ft.map(([k, l]) => `<button data-act="gmodel" data-v="${k}" class="${S.gm === k ? "on" : ""}">${l}</button>`).join("")}</div></div>${S.gbNote ? `<div class="sub" style="margin:0 2px 10px">${esc(S.gbNote)}</div>` : ""}${list.length ? (ui.vd === "list" ? `<div class="list">${list.map(resultCard).join("")}</div>` : `<div class="grid">${list.map(resultCard).join("")}</div>`) : '<div class="empty">Nothing found.</div>'}`;
  }
  else body = myUpdatesView();
  if (S.gbList && S.gbMore && !S.loading && !S.gbView) body += `<div class="form" style="margin-top:16px;justify-content:center"><button data-act="gbmore">Load more</button></div>`;
  return `${subs}<div class="form"><input type="text" id="gin" value="${esc(S.gq || '')}" placeholder="Mod name, author:&quot;Name&quot;, GameBanana link, id or /dl/ link" data-enter="fetch" data-in="gq">${ib("fetch", "search", "Search", "", "primary")}${viewSeg("vd")}</div>${body}
`;
}
export const downloadsView = () => pageHead("Downloads", "Search GameBanana (mods and sounds), watch the queue, or install local files") + downloadsInner();
