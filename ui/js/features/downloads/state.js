import { S } from "../../core/state.js";

// State owned by the Downloads feature (the queue itself, S.dls, lives in core/state.js).
Object.assign(S, {
  dlSub: "download",  // queue | download | upload
  gm: "all",          // result filter: all | Mod | Sound
  gq: "", gbPage: 1, browsing: false,
  gbMore: false,      // "Load more" makes sense
  gbNote: "",         // e.g. "No mod by ... was found"
  gbRun: "",          // the search text that produced the shown list ("" = the latest-updates browse); "Load more" continues it
  gAuthorId: null, gAuthorQ: "", // member id of the author link; valid while the search box still holds gAuthorQ
  gbList: null, gbTitle: "", gbView: null, loading: false, // GameBanana results and the open detail page
  gbFrom: "",         // "mods" = detail page was opened from the Mods tab (Back returns there)
  gbOpen: {},         // which collapsible blocks of the detail page are open ("files", "u0", "u1" ...)
  up: [],             // local files waiting to be installed
});
