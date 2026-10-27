import { S } from "../../core/state.js";

// State owned by the Cursor feature.
Object.assign(S, {
  cur: null,  // result of cursor_info (null = not loaded yet)
  cv: 0,      // cache-buster for cursor images
  cq: "",     // library search
  ck: "all",  // library filter: all | cursor | ping | shop | commend
});
