// Actions used by several tabs: list/card switch and opening web links.
import { ui, saveUi } from "../core/state.js";
import { call } from "../core/feedback.js";
import { render } from "../core/render.js";

export default {
  id: "common",
  acts: {
    view: (d) => { ui[d.k] = d.v; saveUi(); render(); },
    openlink: (d) => call("open_link", { url: d.url }).catch(() => {}),
  },
};
