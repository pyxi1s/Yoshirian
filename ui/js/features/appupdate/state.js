import { S } from "../../core/state.js";

// State owned by the App update feature (S.appUpdate itself lives in core/state.js: the title bar reads it).
Object.assign(S, {
  appUpdChecking: false, // a check is running
  appUpdError: "",       // message of the last failed check ("" = ok)
});
