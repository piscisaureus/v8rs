const assert = require("assert");

class Chain extends Array {
  constructor(lt, ...scopes) {
    super(...scopes);
    this.lt = lt ?? 0;
  }
  clone() {
    return new Chain(this.lt, ...this);
  }
  find(kind) {
    return this.filter(s => s.kind === kind).pop();
  }
  find_front(kind) {
    return this[0]?.kind === kind ? this[0] : null;
  }
  remove(kind) {
    return new Chain(this.lt, ...this.filter(s => s.kind !== kind));
  }
  get_default_parent() {
    if (
      (this.length >= 1 && this[this.length - 1].kind === "Handle") ||
      (this.length >= 2 &&
        this[this.length - 2].kind === "Handle" &&
        this[this.length - 1].kind === "Escape") ||
      (this.length >= 1 && this[this.length - 1].kind === "TryCatch")
    ) {
      return { kind: "Context" };
    }
  }
  append_default_parent() {
    let d = this.get_default_parent();
    if (!d) return this;
    return new Chain(this.lt, ...this, d);
  }
  add_context() {
    if (!this.find("Handle")) return; // Not without without a HandleScope.
    const lt = this.lt; // ?? 0 + 1;
    const p = new Chain(lt, { kind: "Context" });
    const r = new Chain(lt, ...p, ...this.remove("Context"));
    return [p, r];
  }
  add_handle() {
    if (!this.find("Context") && this.find("Handle")) return; // Without context, only 1 level deep.
    const lt = this.lt + 1;
    const p = new Chain(lt, { kind: "Handle", lt });
    const r = new Chain(
      lt,
      ...p,
      ...this.filter(s => s.kind !== "Handle" && s.kind !== "TryCatch")
    );
    return [p, r];
  }
  add_escapable_handle() {
    if (!this.find("Context")) return; // Not without without a Context.
    const lt = this.lt + 1;
    let escape_lt = this.find("Handle")?.lt;
    if (escape_lt == null) return;
    const p = new Chain(
      lt,
      { kind: "Handle", lt },
      { kind: "Escape", lt: escape_lt }
    );
    const r = new Chain(
      lt,
      ...p,
      ...this.filter(
        s => s.kind !== "Handle" && s.kind !== "Escape" && s.kind !== "TryCatch"
      )
    );
    return [p, r];
  }
  add_try_catch() {
    if (!this.find("Context")) return; // Not without without a Context.
    if (!this.find("Handle")) return; // Not without without a HandleScope.
    if (this.find_front("TryCatch")) return; // No immediate nesting of TryCatch blocks.
    const lt = this.lt + 1;
    const p = new Chain(lt, { kind: "TryCatch", lt });
    const r = new Chain(lt, ...p, ...this.filter(s => s.kind !== "TryCatch"));
    return [p, r];
  }
  *add_all() {
    yield this.add_context();
    yield this.add_handle();
    yield this.add_escapable_handle();
    yield this.add_try_catch();
  }
  gather_all_recursive(chain_map) {
    let key = this.serialize();
    if (chain_map.has(key)) return;
    if (key != null) chain_map.set(key, this);
    [...this.add_all()]
      .filter(Boolean)
      .map(([, c]) => c.gather_all_recursive(chain_map));
  }

  try_deref() {
    if (this.find("TryCatch")) return this.remove("TryCatch");
    if (this.find("Escape")) return this.remove("Escape");
    if (this.find("Context")) return this.remove("Context");
    if (this.find("Handle")) return this.remove("Handle");
  }
  deref() {
    let r = this.try_deref();
    if (r.length === 0) return;
    r.lt = Math.max(0, ...r.map(s => s.lt).filter(Boolean));
    return r;
  }
  get_lts() {
    return [this.lt, ...this].filter(s => s.lt != null);
  }
  gather_lts(lt_set) {
    this.get_lts().forEach(s => lt_set.add(s.lt));
  }
  serialize(named_lts = name_lts(this), inner_ns = "active::") {
    let elide_if_default = (p, d) =>
      p !== d?.kind ? `, ${ns(p, inner_ns)}` : ``;
    if (this.length === 0) return;
    let a = this;
    let scope = a.find("Context") ? "Context" : "()";
    a = a.remove("Context");
    let h_default_parent = a.get_default_parent();
    let h = a.find("Handle");
    a = a.remove("Handle");
    let e = a.find("Escape");
    a = a.remove("Escape");
    scope = e
      ? `EscapableHandleScope<${named_lts.get(h.lt)}, ${named_lts.get(
          e.lt
        )}${elide_if_default(scope, h_default_parent)}>`
      : h
      ? `HandleScope<${named_lts.get(h.lt)}${elide_if_default(
          scope,
          h_default_parent
        )}>`
      : scope;
    let t = a.find("TryCatch");
    let t_default_parent = a.get_default_parent();
    scope = t
      ? `TryCatch<${named_lts.get(t.lt)}${elide_if_default(
          scope,
          t_default_parent
        )}>`
      : scope;
    a = a.remove("TryCatch");
    assert(a.length === 0);
    return scope;
  }
}

function name_lts(...chains) {
  let lts = new Set();
  chains.forEach(c => c.gather_lts(lts));
  let numbered_lts = [...lts].sort((a, b) => b - a);
  return new Map(
    numbered_lts.map((lt, index) => [
      lt,
      "'" + String.fromCharCode("a".charCodeAt(0) + index)
    ])
  );
}

function serialize_lts(named_lts, constrain = true) {
  let prev_lt;
  return [...named_lts.keys()]
    .map(lt => {
      let constraint =
        constrain && prev_lt != null ? named_lts.get(prev_lt) : null;
      prev_lt = lt;
      return named_lts.get(lt) + (constraint ? `: ${constraint}` : ``);
    })
    .join(", ");
}

function ns(scope, prefix) {
  switch (scope) {
    case "":
    case "()":
    case "Context":
      return scope;
    default:
      return prefix + scope;
  }
}

const chain_map = new Map();
new Chain().gather_all_recursive(chain_map);
console.log([...chain_map.keys()]);

const deref_map = new Map();
for (let c1 of chain_map.values()) {
  for (let c2; (c2 = c1.deref()) != null; c1 = c2) {
    const named_lts = name_lts(c1, c2);
    let kv = [c1, c2].map(c => c.serialize(named_lts, "")).filter(Boolean);
    if (kv.length < 2) continue;
    const [k, v] = kv;
    const l = serialize_lts(named_lts, false);
    if (deref_map.has(k)) {
      assert(deref_map.get(k)[0] === v);
    } else {
      deref_map.set(k, [v, l]);
    }
  }
}
console.log(
  [...deref_map]
    .map(
      ([k, [v, l]]) => `\
impl<${l}> Deref for ${k} {
  type Target = ${v};
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Self::Target) }
  }
}

impl<${l}> DerefMut for ${k} {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { &mut *(self as *mut _ as *mut Self::Target) }
  }
}

`
    )
    .join("")
);

let mappings = Object.create(null);
for (const c1 of chain_map.values()) {
  for (let [a, c2] of [...c1.add_all()].filter(Boolean)) {
    a = a.append_default_parent();
    let sa = a.serialize();
    const named_lts = name_lts(c1, c2);
    let [sc1, sc2] = [c1, c2].map(c => c.serialize(named_lts)).filter(Boolean);
    if (!(sc1 && sc2)) continue;
    let code =
      [
        `impl<${serialize_lts(named_lts)}> Add${sa.replace(
          /\<.*/,
          ""
        )}<${named_lts.get(c2.lt)}> for ${ns(sc1, "active::")} {`,
        `  type NewScope = ${ns(sc2, "alloc::")};`,
        `}`
      ].join("\n") + "\n";
    (mappings[sa] ?? (mappings[sa] = [])).push(code);
  }
}
mappings = Object.entries(mappings)
  .map(([k, v]) => [`\n// ===== ${k} =====\n`, ...v])
  .flat(1);
console.log(mappings.join("\n"));
