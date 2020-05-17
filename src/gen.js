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
    return this.filter((s) => s.kind === kind).pop();
  }
  find_front(kind) {
    return this[0]?.kind === kind ? this[0] : null;
  }
  remove(kind) {
    return new Chain(this.lt, ...this.filter((s) => s.kind !== kind));
  }
  add_context() {
    if (!this.find("Handle")) return; // Not without without a HandleScope.
    // Entering a context does not change the scope lifetime.
    const p = new Chain(this.lt, { kind: "Context" });
    const r = new Chain(this.lt, ...p, ...this.remove("Context"));
    return [p, r];
  }
  add_handle() {
    if (!this.find("Context") && this.find("Handle")) return; // Without context, only 1 level deep.
    const lt = this.lt + 1;
    const p = new Chain(lt, { kind: "Handle", lt });
    const r = new Chain(
      lt,
      ...p,
      ...this.filter((s) => s.kind !== "Handle" && s.kind !== "TryCatch")
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
        (s) =>
          s.kind !== "Handle" && s.kind !== "Escape" && s.kind !== "TryCatch"
      )
    );
    return [p, r];
  }
  add_try_catch() {
    if (!this.find("Context")) return; // Not without without a Context.
    if (!this.find("Handle")) return; // Not without without a HandleScope.
    const lt = this.lt + 1;
    const p = new Chain(lt, { kind: "TryCatch", lt });
    const r = new Chain(lt, ...p, ...this.filter((s) => s.kind !== "TryCatch"));
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
    r.lt = Math.max(0, ...r.map((s) => s.lt).filter(Boolean));
    return r;
  }

  gather_lts(lt_set) {
    lt_set.add(this.lt);
    this.filter((s) => s.lt != null).forEach((s) => lt_set.add(s.lt));
  }
  serialize_nice(named_lts) {
    let a = this;
    let scope = a.find("Context") ? "Context" : "()";
    a = a.remove("Context");
    let e = a.find("Escape");
    a = a.remove("Escape");
    let h = a.find("Handle");
    a = a.remove("Handle");
    scope = e
      ? `EscapableHandleScope<${named_lts.get(h.lt)}, ${named_lts.get(
          e.lt
        )}, ${scope}>`
      : h
      ? `HandleScope<${named_lts.get(h.lt)}, ${scope}>`
      : scope;
    let t = a.find("TryCatch");
    scope = t ? `TryCatch<${named_lts.get(t.lt)}, ${scope}>` : scope;
    a = a.remove("TryCatch");
    assert(a.length === 0);
    return scope;
  }
  serialize(named_lts = name_lts(this)) {
    if (this.length === 0) return;
    return this.serialize_nice(named_lts);
  }
}

function name_lts(...chains) {
  let lts = new Set();
  chains.forEach((c) => c.gather_lts(lts));
  let numbered_lts = [...lts].sort((a, b) => b - a);
  return new Map(
    numbered_lts.map((lt, index) => [
      lt,
      "'" + String.fromCharCode("a".charCodeAt(0) + index),
    ])
  );
}

const chain_map = new Map();
new Chain().gather_all_recursive(chain_map);
console.log([...chain_map.keys()]);

const deref_map = new Map();
for (let c1 of chain_map.values()) {
  for (let c2; (c2 = c1.deref()) != null; c1 = c2) {
    const named_lts = name_lts(c1, c2);
    let kv = [c1, c2].map((c) => c.serialize(named_lts)).filter(Boolean);
    if (kv.length < 2) continue;
    const [k, v] = kv;
    if (deref_map.has(k)) {
      assert(deref_map.get(k) === v);
    } else {
      deref_map.set(k, v);
    }
  }
}
console.log([...deref_map].map(([k, v]) => `${k} => ${v}`));

console.log();

const mappings = [];
for (const c1 of chain_map.values()) {
  for (const [a, c2] of [...c1.add_all()].filter(Boolean)) {
    const named_lts = name_lts(a, c1, c2);
    let cxxx = [a, c1, c2].map((c) => c.serialize(named_lts)).filter(Boolean);
    if (cxxx.length < 3) continue;
    console.log(`${cxxx[0]} + ${cxxx[1]} => ${cxxx[2]}`);
  }
}
