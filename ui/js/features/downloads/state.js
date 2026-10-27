import { S } from "../../core/state.js";

// State owned by the Downloads feature (the queue itself, S.dls, lives in core/state.js).
Object.assign(S, {
  dlSub: "download",  // queue | download | upload
  gm: "all",          // result filter: all | Mod | Sound
  gq: "", gbMode: "", gbPage: 1, browsing: false,
  gbList: null, gbTitle: "", gbView: null, loading: false, // GameBanana results and the open detail page
  gbFrom: "",         // "mods" = detail page was opened from the Mods tab (Back returns there)
  up: [],             // local files waiting to be installed
});
