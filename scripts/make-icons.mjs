// Rebuilds the installer / window icons in `assets/icons/` from `assets/icon.svg` (or .png, 1024x1024 or more).
// Usage: replace assets/icon.svg, then run `npm run icons`.
import { cpSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const src = ["icon.svg", "icon.png"].map((f) => join(root, "assets", f)).find(existsSync);
if (!src) throw new Error("Put assets/icon.svg (or icon.png) first");
const tmp = join(root, ".icons-tmp");
rmSync(tmp, { recursive: true, force: true });

const r = spawnSync("npx", ["tauri", "icon", src, "--output", tmp], { cwd: root, stdio: "inherit", shell: true });
if (r.status !== 0) throw new Error("tauri icon failed");

// `tauri icon` also makes phone / store icons; only these are used
const keep = ["32x32.png", "128x128.png", "128x128@2x.png", "icon.png", "icon.ico"];
const out = join(root, "assets", "icons");
mkdirSync(out, { recursive: true });
for (const f of keep) cpSync(join(tmp, f), join(out, f));
rmSync(tmp, { recursive: true, force: true });
console.log("assets/icons/ updated:", keep.join(", "));
