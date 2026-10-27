// Safe renderer for GameBanana's HTML descriptions (tables, lists, headings, links, images ...).
// The input is untrusted, so the HTML is never inserted as-is: it is parsed, and a NEW tree is built
// from a whitelist of tags and attributes. Scripts, styles, event handlers and iframes are dropped.
import { esc } from "./dom.js";

const KEEP = new Set(["p", "br", "hr", "b", "strong", "i", "em", "u", "s", "strike", "del", "sub", "sup", "small", "code", "pre", "kbd", "blockquote",
  "ul", "ol", "li", "dl", "dt", "dd", "h1", "h2", "h3", "h4", "h5", "h6", "table", "thead", "tbody", "tfoot", "tr", "th", "td", "caption", "span", "div", "img", "a"]);
const DROP_WITH_CONTENT = new Set(["script", "style", "iframe", "object", "embed", "noscript", "template", "svg", "math", "form", "textarea", "select"]);
const SAFE_URL = /^https?:\/\//i;

function clean(node) {
  let out = "";
  for (const n of node.childNodes) {
    if (n.nodeType === 3) { out += esc(n.nodeValue); continue; }
    if (n.nodeType !== 1) continue;
    const t = n.tagName.toLowerCase();
    if (DROP_WITH_CONTENT.has(t)) continue;
    const inner = clean(n);
    if (!KEEP.has(t)) { out += inner; continue; } // unknown tag: keep its text, drop the tag
    if (t === "br" || t === "hr") { out += `<${t}>`; continue; }
    if (t === "img") {
      const src = n.getAttribute("src") || "";
      if (SAFE_URL.test(src)) out += `<img src="${esc(src)}" alt="${esc(n.getAttribute("alt") || "")}" loading="lazy">`;
      continue;
    }
    if (t === "a") {
      const h = n.getAttribute("href") || "";
      out += SAFE_URL.test(h) ? `<a href="#" data-act="openlink" data-url="${esc(h)}">${inner}</a>` : inner;
      continue;
    }
    let attrs = "";
    if (t === "td" || t === "th") for (const k of ["colspan", "rowspan"]) { const v = parseInt(n.getAttribute(k), 10); if (v > 1 && v < 50) attrs += ` ${k}="${v}"`; }
    if (t === "table") { out += `<div class="rich-scroll"><table>${inner}</table></div>`; continue; } // wide tables scroll sideways
    out += `<${t}${attrs}>${inner}</${t}>`;
  }
  return out;
}

/** HTML string -> sanitized HTML string, ready to put inside `<div class="rich">`. */
export function richHtml(html) {
  return clean(new DOMParser().parseFromString(html || "", "text/html").body);
}

/** Plain text of an HTML snippet (decodes entities such as &amp;). */
export function plainText(html) {
  return (new DOMParser().parseFromString(html || "", "text/html").body.textContent || "").trim();
}
