// Small UI helpers: toast, modal dialogs and the backend call wrapper.
import { invoke, dialogApi } from "./tauri.js";
import { $, esc } from "./dom.js";

let tt;
export function toast(m) {
  const t = $("#toast");
  t.textContent = String(m);
  t.classList.add("on");
  clearTimeout(tt);
  tt = setTimeout(() => t.classList.remove("on"), 3400);
}

/** invoke() that shows errors as a toast and writes them to the app log. Still throws, so callers can stop. */
export async function call(cmd, args) {
  try { return await invoke(cmd, args); } catch (er) { toast(er); invoke("log_ui", { level: "error", msg: `${cmd}: ${er}` }).catch(() => {}); throw er; }
}

export function modal(html, setup, wide) {
  return new Promise((res) => {
    const d = document.createElement("dialog");
    if (wide) d.className = "wide";
    d.innerHTML = html;
    document.body.appendChild(d);
    d.addEventListener("close", () => { d.remove(); res(d.returnValue); });
    d.showModal();
    setup && setup(d);
  });
}
export const askConfirm = async (msg) => (await modal(`<h3>Confirm</h3><p style="white-space:pre-wrap;word-break:break-all">${esc(msg)}</p><form method="dialog" class="btns"><button value="no">Cancel</button><button class="primary" value="yes">OK</button></form>`)) === "yes";
export async function askText(title, val = "") {
  let out = null;
  await modal(`<h3>${esc(title)}</h3><input type="text" id="ask" value="${esc(val)}" style="width:100%"><form method="dialog" class="btns"><button>Cancel</button><button class="primary" value="ok">OK</button></form>`, (d) => {
    const i = d.querySelector("#ask"); i.focus(); i.select();
    d.querySelector("form").addEventListener("submit", (ev) => { if (ev.submitter && ev.submitter.value === "ok") out = i.value.trim(); });
    i.addEventListener("keydown", (ev) => { if (ev.key === "Enter") { ev.preventDefault(); out = i.value.trim(); d.close("ok"); } });
  });
  return out;
}
export const pickImage = () => dialogApi.open({ filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }] });
