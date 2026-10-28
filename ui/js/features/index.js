// The list of features. Order = order of the sidebar tabs.
//   Add a feature : create features/<name>/index.js (see core/registry.js for the shape) and add it here.
//   Remove one    : delete its line (and its folder). Nothing else needs to change.
import common from "./common.js";
import mods from "./mods/index.js";
import downloads from "./downloads/index.js";
import cursor from "./cursor/index.js";
import settings from "./settings/index.js";
import appupdate from "./appupdate/index.js";

export const FEATURES = [common, mods, downloads, cursor, settings, appupdate];
