// Feature registry. Every feature is a plain object (see features/index.js):
//
//   {
//     id: "cursor",
//     tab:  { label, icon, badge?(), beforeEnter?(), onEnter?() },   // optional: shows up in the sidebar
//     view: () => html,                                              // required when `tab` is set
//     acts: { name: (dataset, el, event) => ... },                   // <button data-act="name">  (click)
//     changes: { name: (dataset, el) => ... },                       // <input data-ch="name">    (change)
//     inputs:  { name: (dataset, el) => ... },                       // <input data-in="name">    (input)
//     init?: () => void,                                             // one-time wiring: listeners, backend events
//   }
//
// Add a feature = create the object and list it in features/index.js. Remove it = delete that line.
export const ACTS = {};
export const CHG = {};
export const INP = {};
const features = [];

function merge(into, from, kind, id) {
  for (const [k, fn] of Object.entries(from || {})) {
    if (into[k]) console.warn(`[registry] duplicate ${kind} "${k}" (feature "${id}")`);
    into[k] = fn;
  }
}

export function register(list) {
  for (const f of list) {
    features.push(f);
    merge(ACTS, f.acts, "action", f.id);
    merge(CHG, f.changes, "change handler", f.id);
    merge(INP, f.inputs, "input handler", f.id);
  }
}

export const tabFeatures = () => features.filter((f) => f.tab);
export const tabFeature = (id) => features.find((f) => f.tab && f.id === id);
export const initFeatures = () => features.forEach((f) => f.init && f.init());

/** Run an action by name — lets a feature trigger another feature's action without importing it. */
export const runAct = (name, data = {}, el = null, ev = null) => (ACTS[name] ? ACTS[name](data, el, ev) : undefined);
