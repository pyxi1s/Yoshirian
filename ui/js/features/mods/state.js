import { S } from "../../core/state.js";

// State owned by the Mods feature (S.mods / S.profiles / S.patched live in core/state.js because other features read them).
Object.assign(S, {
  q: "",            // search box
  mf: "all",        // filter: all | on | off
  modView: null,    // id of the mod whose detail page is open
  updates: {},      // mod id -> remote "updated" timestamp (update available)
  updCount: 0, updChecked: false,
  sndLists: {},     // mod id -> sounds inside its vpk
  sndSel: {},       // mod id -> selected sound index
});
