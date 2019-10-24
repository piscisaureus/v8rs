use std::cell::Cell;
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::drop;
use std::mem::transmute;
use std::mem::ManuallyDrop;
use std::mem::MaybeUninit;

// Dummy placeholders representing raw v8 objects.
struct V8Isolate {}
struct V8HandleScope {}

trait ScopeImpl {}

// Scope that controls access to the Isolate and active HandleScope.
struct ScopeData<'a, S> {
  label: &'static str,
  parent_scope: PhantomData<&'a ()>,
  ref_count: usize,
  v8_object: S, // container for raw v8 Isolate or HandleScope.
}

impl<'a, S> Drop for ScopeData<'a, S> {
  fn drop(&mut self) {
    println!("drop inner {}", self.label);
  }
}

type ScopeInner<'a, S> = UnsafeCell<ManuallyDrop<ScopeData<'a, S>>>;

// Scope that controls access to the Isolate and active HandleScope.
struct Scope<'a, S>(ScopeInner<'a, S>);

impl<'a, S> ScopeImpl for Scope<'a, S> {}

impl<'a> Scope<'a, V8Isolate> {
  fn new_isolate() -> Self {
    println!("new isolate");
    Self(UnsafeCell::new(ManuallyDrop::new(ScopeData {
      label: "isolate",
      parent_scope: PhantomData,
      ref_count: 1,
      v8_object: V8Isolate {},
    })))
  }
}

impl<'a, S> Scope<'a, S> {
  fn new_handle_scope<'n>(
    &'n mut self,
    label: &'static str,
  ) -> Scope<'n, V8HandleScope> {
    println!("new {}", label);
    Self(UnsafeCell::new(ManuallyDrop::new(ScopeData {
      label,
      parent_scope: PhantomData,
      ref_count: 1,
      v8_object: V8HandleScope {},
    })))
  }

  fn data(&self) -> &ScopeData<'a, S> {
    unsafe { &mut *self.0.get() }
  }

  fn ref_count_inc(inner: &ScopeInner<'a, S>) {
    let data = unsafe { &mut *inner.get() };
    data.ref_count += 1;
    println!("  ++{} -> {}", data.label, data.ref_count);
  }

  fn ref_count_dec(inner: &ScopeInner<'a, S>) {
    let data = unsafe { &mut *inner.get() };
    data.ref_count -= 1;
    println!("  --{} -> {}", data.label, data.ref_count);
    if data.ref_count == 0 {
      unsafe { ManuallyDrop::drop(data) }
    }
  }
}

impl<'a, S> Drop for Scope<'a, S> {
  fn drop(&mut self) {
    println!("drop {}", self.data().label);
    Self::ref_count_dec(&self.0);
  }
}

struct ScopeRef<'a, S>(&'a ScopeInner<'a, S>);

impl<'a, S> ScopeRef<'a, S> {
  fn new(scope: &mut Scope<'a, S>) -> Self {
    Scope::ref_count_inc(&scope.0);
    Self(unsafe { transmute(&scope.0) })
  }
}

impl<'a, S> Drop for ScopeRef<'a, S> {
  fn drop(&mut self) {
    Scope::ref_count_dec(self.0);
  }
}

struct Local<'sc> {
  label: &'static str,
  val: i32,
  parent_scope: ScopeRef<'sc, V8HandleScope>,
}

impl<'sc> Local<'sc> {
  fn new(scope: &mut Scope<'sc, V8HandleScope>, label: &'static str) -> Self {
    println!("new {}", label);
    Self {
      label,
      val: 0,
      parent_scope: ScopeRef::new(scope),
    }
  }

  fn alive(&self) {
    println!("alive {}", self.label);
  }
}

impl<'sc> Drop for Local<'sc> {
  fn drop(&mut self) {
    println!("drop {}", self.label);
  }
}

#[allow(unused_variables)]
fn main() {
  let mut isolate = Scope::new_isolate();

  let mut scope1 = isolate.new_handle_scope("scope1");
  let local_a_in_scope1 = Local::new(&mut scope1, "local_a_in_scope1");
  let local_b_in_scope1 = Local::new(&mut scope1, "local_b_in_scope1");

  {
    let mut scope2 = scope1.new_handle_scope("scope2");
    let local_a_in_scope2 = Local::new(&mut scope2, "local_a_in_scope2");
    let local_b_in_scope2 = Local::new(&mut scope2, "local_b_in_scope2");

    // fail: scope1 is made inaccessible by scope2's existence.
    //F let mut _fail = scope1.new_handle_scope();
    // fail: same reason.
    //F let _fail = Local::new(scope1);

    {
      let local_in_scope3;
      let mut scope3 = scope2.new_handle_scope("scope3");
      local_in_scope3 = Local::new(&mut scope3, "local_in_scope3");

      //F let _fail = Local::new(scope1); // fail: scope1 locked by scope2
      //F let _fail = Local::new(scope2); // fail: scope2 locked by scope3

      // The borrow checker allows us to drop a scope while a local that
      // is contained in it is still alive. To stay safe, `Scope<T>`
      // maintains a reference counter. The scope's interior will not
      // actually be dropped until the last local goes out of scope.
      drop(scope3);

      // fail: scope2 still locked because local_in_scope3 is alive,
      // so scope3 must be alive.
      //F let _fail = Local::new(scope2);

      local_in_scope3.alive();

      // pass: after dropping local_in_scope3, scope2 can be used again.
      drop(local_in_scope3);
      let local_c_in_scope2 = Local::new(&mut scope2, "local_c_in_scope2");
    }

    // fail: scope1 not accessible, because local_a_in_scope2 is keeping
    // scope2 alive.
    // let _fail = Local::new(scope1);

    local_a_in_scope2.alive();

    // pass: scope2 and all it's locals dropped, scope1 accessible again.
    drop(local_a_in_scope2);
    drop(local_b_in_scope2);
    drop(scope2);
    let local_c_in_scope1 = Local::new(&mut scope1, "local_c_in_scope1");
  }

  let local_d_in_scope1 = Local::new(&mut scope1, "local_d_in_scope1");
  local_a_in_scope1.alive();

  {
    let mut scope4 = scope1.new_handle_scope("scope4");
    let scope5 = scope4.new_handle_scope("scope5");
    //F drop(scope4);
  }
}
