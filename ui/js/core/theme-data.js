// Theme definition: editable variables, built-in presets and font lists (pure data).

export const VARS = [
  ["Colors", [["--bg", "Background", "color"], ["--side", "Sidebar", "color"], ["--panel", "Panel", "color"], ["--card", "Card", "color"], ["--border", "Border", "color"], ["--text", "Text", "color"], ["--sub", "Muted text", "color"], ["--accent", "Accent", "color"], ["--accent2", "Accent 2 (gradient)", "color"], ["--danger", "Danger", "color"], ["--ok", "Success", "color"], ["--warn", "Warning", "color"], ["--on-accent", "Text on accent", "color"], ["--danger-bg", "Delete button", "color"]]],
  ["Layout", [["--radius", "Corner radius", "range", 0, 28, "px", 1], ["--gap", "Spacing", "range", 4, 32, "px", 1], ["--sidew", "Sidebar width", "range", 160, 320, "px", 2], ["--cardw", "Card width", "range", 160, 380, "px", 5], ["--scale", "UI scale", "range", 0.8, 1.4, "", 0.05]]],
  ["Text", [["--fs", "Font size", "range", 11, 20, "px", 1], ["--fw-head", "Heading weight", "range", 400, 800, "", 100]]],
  ["Background image", [["--bgop", "Image opacity", "range", 0, 1, "", 0.05], ["--blur", "Image blur", "range", 0, 30, "px", 1]]],
];
export const base = { "--radius": 12, "--gap": 16, "--sidew": 240, "--cardw": 218, "--scale": 1, "--fs": 14, "--fw-head": 600, "--bgop": 0.25, "--blur": 0 };
export const mk = (bgc, side, panel, card, border, text, sub, a1, a2, extra = {}) => ({ ...base, "--bg": bgc, "--side": side, "--panel": panel, "--card": card, "--border": border, "--text": text, "--sub": sub, "--accent": a1, "--accent2": a2, "--on-accent": "#ffffff", "--danger": "#ff6b6b", "--danger-bg": "#9b2226", "--ok": "#4ade80", "--warn": "#fbbf24", ...extra });
export const PRESETS = {
  Obsidian: mk("#0b0908", "#0e0c0a", "#191512", "#15110e", "#2b241d", "#f1eadc", "#a1978a", "#ead9b0", "#f6ecd0", { "--on-accent": "#1b140a", "--danger-bg": "#8c1f1f", "--ok": "#8bd17c", "--warn": "#f0b429" }),
  Slate: mk("#10131a", "#141923", "#1a2030", "#1e2535", "#2d3548", "#eef1f8", "#a0aabf", "#6c8cff", "#b47cff"),
  Daylight: mk("#f3f5f9", "#ffffff", "#eef1f6", "#ffffff", "#dbe0ea", "#1b2130", "#5f6b82", "#4f6bff", "#8b5cf6", { "--danger": "#e5484d", "--ok": "#16a34a", "--warn": "#d97706" }),
  Ocean: mk("#0b141c", "#0f1b25", "#14232f", "#182a38", "#254052", "#e8f3f9", "#8fb0c2", "#38bdf8", "#22d3ee", { "--on-accent": "#062030" }),
  Ember: mk("#17100e", "#1d1411", "#261a16", "#2c1f1a", "#43302a", "#fbefe9", "#c0a094", "#ff7a45", "#ff4d6d"),
  Forest: mk("#0a130e", "#0e1914", "#14231b", "#182a20", "#25402f", "#e6f3ea", "#8fb09c", "#3ddc97", "#a3e635", { "--on-accent": "#04210f" }),
  Graphite: mk("#121212", "#181818", "#1f1f1f", "#242424", "#333333", "#eeeeee", "#9a9a9a", "#8ab4f8", "#f28b82", { "--on-accent": "#0b1a33" }),
  Paper: mk("#f6f1e7", "#fbf7ee", "#efe8d8", "#fffdf7", "#ddd2bb", "#2b2418", "#7a6d55", "#b45309", "#d97706", { "--danger": "#c2410c", "--ok": "#15803d", "--warn": "#b45309" }),
};
export const HEAD_FONTS = ["Bahnschrift SemiCondensed", "Bahnschrift", "Segoe UI", "Arial Narrow", "Impact", "Georgia", "Consolas"];
export const FONTS = ["Segoe UI", "Inter", "system-ui", "Arial", "Verdana", "Tahoma", "Trebuchet MS", "Georgia", "Times New Roman", "Comic Sans MS", "Consolas", "Courier New"];
export const newTheme = (n = "Graphite") => ({ name: n, vars: { ...PRESETS[n] }, font: "Segoe UI", headFont: "Bahnschrift SemiCondensed", mono: "Consolas", fontFile: null, bgImage: null, thumbr: "8/7", anim: true });
