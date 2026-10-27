// Sound previews: fetched by the backend, decoded to a waveform, played with <audio>.
// Keys: "https://..." (GameBanana preview) or "vpk:<modId>:<path inside the mod>".
import { invoke } from "../core/tauri.js";
import { ui } from "../core/state.js";
import { esc } from "../core/dom.js";
import { ic } from "../core/icons.js";
import { toast } from "../core/feedback.js";

export const MIME = { mp3: "audio/mpeg", ogg: "audio/ogg", wav: "audio/wav", m4a: "audio/mp4", flac: "audio/flac" };
export const SND = new Map(); // decoded sounds, keyed by url ("vpk:<modId>:<path>" for sounds inside an installed mod)
let AUD = { audio: null, url: "", raf: 0 };
export const fmtTime = (t) => `${Math.floor((t || 0) / 60)}:${String(Math.floor((t || 0) % 60)).padStart(2, "0")}`;
export function getSound(url) {
  if (!SND.has(url)) {
    SND.set(url, (async () => {
      let raw;
      if (url.startsWith("vpk:")) { const [, id, ...rest] = url.split(":"); raw = await invoke("vpk_sound_data", { id, path: rest.join(":") }); }
      else raw = { data: await invoke("fetch_audio", { url }), mime: "" };
      const bytes = Uint8Array.from(atob(raw.data), (c) => c.charCodeAt(0));
      const ext = (url.split("?")[0].split(".").pop() || "mp3").toLowerCase();
      const blobUrl = URL.createObjectURL(new Blob([bytes], { type: raw.mime || MIME[ext] || "audio/mpeg" }));
      let peaks = null, dur = 0;
      try {
        const ctx = new (window.AudioContext || window.webkitAudioContext)();
        const buf = await ctx.decodeAudioData(bytes.buffer.slice(0));
        dur = buf.duration;
        const ch = buf.getChannelData(0), n = 400, step = Math.max(1, Math.floor(ch.length / n));
        peaks = [];
        for (let i = 0; i < n; i++) { let m = 0; for (let j = i * step; j < Math.min(ch.length, (i + 1) * step); j += Math.max(1, step >> 6)) m = Math.max(m, Math.abs(ch[j])); peaks.push(m); }
        const mx = Math.max(...peaks, 0.001); peaks = peaks.map((p) => p / mx);
        ctx.close();
      } catch (_) {}
      return { blobUrl, peaks, dur };
    })().catch((er) => { SND.delete(url); throw er; }));
  }
  return SND.get(url);
}
export function drawWave(cv, peaks, prog) {
  const dpr = window.devicePixelRatio || 1, w = cv.clientWidth, h = cv.clientHeight;
  if (!w) return;
  if (cv.width !== Math.round(w * dpr)) { cv.width = Math.round(w * dpr); cv.height = Math.round(h * dpr); }
  const g = cv.getContext("2d"), cs = getComputedStyle(document.documentElement);
  const on = cs.getPropertyValue("--accent").trim(), off = cs.getPropertyValue("--sub").trim();
  g.clearRect(0, 0, cv.width, cv.height);
  const bw = 3 * dpr, gap = 1.5 * dpr, cols = Math.floor(cv.width / (bw + gap)), n = peaks ? peaks.length : 0;
  for (let i = 0; i < cols; i++) {
    const p = n ? peaks[Math.min(n - 1, Math.floor((i / cols) * n))] : 0.06, bh = Math.max(2 * dpr, p * cv.height * 0.95);
    const played = i / cols < prog;
    g.globalAlpha = played ? 1 : 0.45; g.fillStyle = played ? on : off;
    g.fillRect(i * (bw + gap), (cv.height - bh) / 2, bw, bh);
  }
  g.globalAlpha = 1;
}
export function updateWaves() {
  const a = AUD.audio;
  document.querySelectorAll(".wave").forEach((el) => {
    const cur = a && AUD.url === el.dataset.url, dur = cur && isFinite(a.duration) ? a.duration : el._dur || 0;
    drawWave(el.querySelector("canvas"), el._peaks, cur && dur ? a.currentTime / dur : 0);
    el.querySelector(".wtime").textContent = el._ready ? `${fmtTime(cur ? a.currentTime : 0)} / ${fmtTime(dur)}` : el._err || "Loading…";
  });
  document.querySelectorAll('[data-act="playsnd"]').forEach((b) => { const on = a && !a.paused && b.dataset.url && AUD.url === b.dataset.url; b.innerHTML = ic(on ? "pause" : "play"); });
  document.querySelectorAll('[data-act="playvpk"]').forEach((b) => { const on = a && !a.paused && AUD.url.startsWith(`vpk:${b.dataset.id}:`); b.innerHTML = ic(on ? "pause" : "play"); });
}
export function mountWaves() {
  document.querySelectorAll(".wave").forEach((el) => {
    const url = el.dataset.url;
    el.querySelector("canvas").onclick = (ev) => {
      if (AUD.audio && AUD.url === url && isFinite(AUD.audio.duration)) { const r = ev.currentTarget.getBoundingClientRect(); AUD.audio.currentTime = ((ev.clientX - r.left) / r.width) * AUD.audio.duration; updateWaves(); }
      else playSound(url);
    };
    getSound(url).then((s) => { el._peaks = s.peaks; el._dur = s.dur; el._ready = true; updateWaves(); }).catch(() => { el._err = "Preview unavailable"; updateWaves(); });
  });
  updateWaves();
}
export function stopSound() {
  if (AUD.audio) { AUD.audio.pause(); AUD.audio.currentTime = 0; cancelAnimationFrame(AUD.raf); }
  AUD = { audio: null, url: "", raf: 0 };
  updateWaves();
}
export async function playSound(url) {
  if (AUD.audio && AUD.url === url) { AUD.audio.paused ? AUD.audio.play() : AUD.audio.pause(); return; }
  if (AUD.audio) { AUD.audio.pause(); cancelAnimationFrame(AUD.raf); }
  let snd;
  try { if (!SND.has(url)) toast("Loading preview…"); snd = await getSound(url); } catch (er) { return toast("Could not load the preview: " + er); }
  const a = new Audio(snd.blobUrl);
  a.volume = ui.volume;
  AUD = { audio: a, url, raf: 0 };
  const loop = () => { updateWaves(); if (AUD.audio === a && !a.paused) AUD.raf = requestAnimationFrame(loop); };
  a.onplay = () => { loop(); };
  a.onpause = a.onended = updateWaves;
  a.play().catch(() => toast("Could not play the preview"));
}
export const waveHtml = (url) => `<div class="wave" data-url="${esc(url)}"><button class="ib primary" data-act="playsnd" data-url="${esc(url)}" title="Play / pause">${ic("play")}</button><canvas></canvas><span class="wtime">Loading…</span></div>`;
/** Apply ui.volume to the sound that is playing. */
export const applyVolume = () => { if (AUD.audio) AUD.audio.volume = ui.volume; };
