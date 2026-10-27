// Syntax highlighting for the built-in gameinfo.gi / autoexec.cfg editors.
import { esc } from "../../core/dom.js";

export function hlLine(line, kind) {
  const rx = /(\/\/.*$)|("(?:[^"\\]|\\.)*"?)|([{}])|(-?\d+(?:\.\d+)?(?![\w.]))|([A-Za-z_$#+][^\s"{}]*)|(\s+)|(.)/g;
  let out = "", first = true, m;
  while ((m = rx.exec(line))) {
    const t = m[0], e = esc(t);
    if (m[1]) out += `<i class="c">${e}</i>`;
    else if (m[2]) { out += `<i class="${first ? "k" : "s"}">${e}</i>`; first = false; }
    else if (m[3]) out += `<i class="b">${t}</i>`;
    else if (m[4]) { out += `<i class="n">${e}</i>`; first = false; }
    else if (m[5]) { out += first ? `<i class="k">${e}</i>` : /[\\\/.:]/.test(t) ? `<i class="p">${e}</i>` : e; first = false; }
    else out += e;
  }
  return kind === "gameinfo" && /citadel\/addons/.test(line) ? `<i class="hot">${out}</i>` : out;
}
export const hlCode = (text, kind) => text.split("\n").map((l) => hlLine(l, kind)).join("\n") + "\n";
