use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::marker::PhantomPinned;
use std::mem::drop;
use std::mem::transmute;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ptr::null;

mod raw {
  use super::*;

  #[derive(Debug)]
  pub(crate) struct HandleScope {
    _pin: PhantomPinned,
  }

  impl HandleScope {
    pub fn construct_in_place(buf: &mut MaybeUninit<Self>) {
      println!("construct HandleScope {:?}", buf as *const _ as *const Self);
    }
    pub fn destruct_in_place(&mut self) {
      println!("destruct HandleScope {:?}", self as *const Self);
    }

    pub unsafe fn cast_mut(buf: &mut MaybeUninit<Self>) -> &mut Self {
      &mut *(buf as *mut std::mem::MaybeUninit<raw::HandleScope>
        as *mut raw::HandleScope)
    }
  }

  #[repr(transparent)]
  #[derive(Debug)]
  pub(crate) struct Local<T>(&'static mut T)
  where
    T: 'static;
}

// Dummy placeholders representing raw v8 objects.
struct Isolate {}

trait HandleScopeParent {}
impl<'a> HandleScopeParent for HandleScope<'a> {}
impl HandleScopeParent for Isolate {}

// HandleScope that controls access to the Isolate and active HandleScope.
struct HandleScopeData<'a> {
  label: &'static str,
  addr: *const Self,
  refs: usize,
  raw: MaybeUninit<raw::HandleScope>,
  parent: PhantomData<&'a ()>,
}

impl<'a> HandleScopeData<'a> {
  fn new(label: &'static str) -> UnsafeCell<Self> {
    UnsafeCell::new(Self {
      label,
      addr: null(),
      refs: 0,
      raw: MaybeUninit::uninit(),
      parent: PhantomData,
    })
  }

  #[inline(always)]
  fn panic_if_moved(&self) {
    if self.refs == 0 {
      debug_assert!(self.addr.is_null());
    } else if self.addr != self {
      panic!("An HandleScope should not be moved");
    }
  }

  #[allow(clippy::mut_from_ref)]
  unsafe fn get_mut(cell: &UnsafeCell<Self>) -> &mut Self {
    let data = &mut *cell.get();
    data.panic_if_moved();
    data
  }

  fn add_ref(&mut self) {
    self.panic_if_moved();
    if self.refs == 0 {
      raw::HandleScope::construct_in_place(&mut self.raw);
      self.addr = self;
    }
    self.refs += 1;
    println!("  ++{} -> {}", self.label, self.refs);
  }

  fn drop_ref(&mut self) {
    self.panic_if_moved();
    self.refs -= 1;
    if self.refs == 0 {
      unsafe { raw::HandleScope::cast_mut(&mut self.raw) }.destruct_in_place();
      self.addr = null();
    }
    println!("  --{} -> {}", self.label, self.refs);
  }
}

struct HandleScope<'a>(UnsafeCell<HandleScopeData<'a>>);

impl<'a> HandleScope<'a> {
  pub fn new<P>(_parent: &'a mut P, label: &'static str) -> Self
  where
    P: HandleScopeParent + 'a,
  {
    println!("new {}", label);
    Self(HandleScopeData::new(label))
  }
}

impl<'a> Deref for HandleScope<'a> {
  type Target = HandleScopeData<'a>;
  fn deref(&self) -> &Self::Target {
    unsafe { &mut *self.0.get() }
  }
}

impl<'a> Drop for HandleScope<'a> {
  fn drop(&mut self) {
    println!("drop {}", self.label);
  }
}

struct HandleScopeRef<'a>(&'a UnsafeCell<HandleScopeData<'a>>);

impl<'a> HandleScopeRef<'a> {
  fn new(scope: &mut HandleScope<'a>) -> Self {
    unsafe { HandleScopeData::get_mut(&scope.0) }.add_ref();
    Self(unsafe { transmute(&scope.0) })
  }
}

impl<'a> Drop for HandleScopeRef<'a> {
  fn drop(&mut self) {
    unsafe { HandleScopeData::get_mut(self.0) }.drop_ref();
  }
}

struct Local<'sc> {
  label: &'static str,
  _handle_scope: HandleScopeRef<'sc>,
}

impl<'sc> Local<'sc> {
  fn new(scope: &mut HandleScope<'sc>, label: &'static str) -> Self {
    println!("new {}", label);
    Self {
      label,
      _handle_scope: HandleScopeRef::new(scope),
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
  let mut isolate = Isolate {};
  let mut scope1 = HandleScope::new(&mut isolate, "scope1");

  let local_a_in_scope1 = Local::new(&mut scope1, "local_a_in_scope1");
  let local_b_in_scope1 = Local::new(&mut scope1, "local_b_in_scope1");

  {
    let mut scope2 = HandleScope::new(&mut scope1, "scope2");
    let local_a_in_scope2 = Local::new(&mut scope2, "local_a_in_scope2");
    let local_b_in_scope2 = Local::new(&mut scope2, "local_b_in_scope2");

    // fail: scope1 is made inaccessible by scope2's existence.
    //F let mut _fail = HandleScope::new(&mut scope1);
    // fail: same reason.
    //F let _fail = Local::new(scope1);

    {
      let local_in_scope3;
      let mut scope3 = HandleScope::new(&mut scope2, "scope3");
      local_in_scope3 = Local::new(&mut scope3, "local_in_scope3");

      //F let _fail = Local::new(scope1); // fail: scope1 locked by scope2
      //F let _fail = Local::new(scope2); // fail: scope2 locked by scope3

      // Should be allowed.
      drop(local_b_in_scope2);

      // The borrow checker allows us to drop a scope while a local that
      // is contained in it is still alive. To stay safe, `HandleScope<T>`
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
    drop(scope2);
    let local_c_in_scope1 = Local::new(&mut scope1, "local_c_in_scope1");
  }

  let local_d_in_scope1 = Local::new(&mut scope1, "local_d_in_scope1");
  local_a_in_scope1.alive();

  {
    let mut scope4a = HandleScope::new(&mut scope1, "scope4");
    let local_in_scope4a = Local::new(&mut scope4a, "local_in_scope4a");
    drop(local_in_scope4a);
    let mut scope4b = Box::new(scope4a);
    let local_in_scope4b = Local::new(&mut scope4b, "local_in_scope4b");
    drop(local_in_scope4b);
    let mut scope4c = *scope4b;
    let local_in_scope4c = Local::new(&mut scope4c, "local_in_scope4c");
  }
}
