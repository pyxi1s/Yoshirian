// Copies the media of the root `assets/` folder into `ui/assets/`, because the window can only load files that
// are inside `ui/` (Tauri's frontendDist). Runs automatically before `tauri dev` / `tauri build` (see tauri.conf.json).
// `assets/` is the ONLY place to put images / icons / sounds. `ui/assets/` is generated: never edit it (it is git-ignored).
// `assets/icons/` is for the installer and the window icon only (src-tauri reads it), so it is not copied.
import { cpSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const from = join(root, "assets");
const to = join(root, "ui", "assets");

if (!existsSync(from)) throw new Error(`Missing folder: ${from}`);
rmSync(to, { recursive: true, force: true }); // no stale files
mkdirSync(to, { recursive: true });
cpSync(from, to, {
  recursive: true,
  filter: (src) => {
    const rel = relative(from, src).split("\\").join("/");
    return rel !== "icons" && !rel.startsWith("icons/") && rel !== "README.md";
  },
});
console.log("assets/ -> ui/assets/");
