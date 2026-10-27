import { S } from "../../core/state.js";

// State owned by the Settings feature.
Object.assign(S, {
  sub: "game", // highlighted section in the left sub-menu
  txt: {},     // text of gameinfo.gi / autoexec.cfg while editing (undefined = not loaded)
  logs: [], info: null,
});
