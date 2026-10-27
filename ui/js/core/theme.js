// Applies ui.theme to the page (CSS variables, fonts, background image).
import { VARS } from "./theme-data.js";
import { ui, saveUi } from "./state.js";
import { $, local } from "./dom.js";
import { toast } from "./feedback.js";

let fontLoaded = null;
/** Forget the loaded custom font so the next applyTheme() loads it again. */
export const resetFontCache = () => { fontLoaded = null; };

export function applyTheme() {
  const t = ui.theme, r = document.documentElement;
  for (const [, items] of VARS) for (const [k, , type, , , unit] of items) r.style.setProperty(k, type === "range" ? t.vars[k] + (unit || "") : t.vars[k]);
  r.style.setProperty("--thumbr", t.thumbr);
  r.style.setProperty("--mono", `"${t.mono}", monospace`);
  r.style.setProperty("--font-head", `"${t.headFont || "Bahnschrift SemiCondensed"}", "Bahnschrift", "Segoe UI", sans-serif`);
  r.dataset.anim = t.anim ? "on" : "off";
  const fam = `"${t.font}", "Segoe UI", system-ui, sans-serif`;
  if (t.fontFile) {
    if (fontLoaded !== t.fontFile) {
      fontLoaded = t.fontFile;
      new FontFace("YoshiCustom", `url("${local(t.fontFile)}")`).load().then((f) => { document.fonts.add(f); r.style.setProperty("--font", `"YoshiCustom", ${fam}`); }).catch(() => toast("Could not load the font file"));
    } else r.style.setProperty("--font", `"YoshiCustom", ${fam}`);
  } else r.style.setProperty("--font", fam);
  $("#bg").style.backgroundImage = t.bgImage ? `url("${local(t.bgImage)}")` : "none";
  saveUi();
}
