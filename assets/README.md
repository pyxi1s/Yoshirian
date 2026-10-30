# assets/ — the one place for images, icons and other media

| what | where |
|---|---|
| App logo (source) | `icon.svg` — used in the title bar and to make the installer icons |
| Installer / window icons | `icons/` — made by `npm run icons` from `icon.svg` (do not edit by hand) |
| Any other image / sound the UI shows | put it here, then use it as `assets/<file>` in `ui/` code |

`ui/assets/` is a generated copy of this folder (made by `scripts/sync-assets.mjs` before `tauri dev` / `tauri build`).
Never edit or add files there: they are deleted and re-copied every time.

Change the logo: replace `icon.svg`, then run `npm run icons`.
