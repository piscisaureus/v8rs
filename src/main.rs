use std::cell::Cell;
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::align_of;
use std::mem::replace;
use std::mem::size_of;
use std::mem::size_of_val;
use std::mem::take;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr;
use std::ptr::drop_in_place;
use std::ptr::null_mut;
use std::rc::Rc;

trait ScopeData {
  type Args;
  fn construct(buf: *mut Self, args: Self::Args);
  fn get_active_scope_slot(active_scope: &mut ActiveScope) -> &mut *mut Self;
}

struct HandleScopeData([usize; 3]);
impl ScopeData for HandleScopeData {
  type Args = ();

  #[inline(never)]
  fn construct(buf: *mut Self, _args: Self::Args) {
    unsafe { ptr::write(buf, Self(Default::default())) }
  }

  #[inline(always)]
  fn get_active_scope_slot(active_scope: &mut ActiveScope) -> &mut *mut Self {
    &mut active_scope.handle_scope
  }
}

struct EscapeSlotData([usize; 1]);
impl ScopeData for EscapeSlotData {
  type Args = ();

  #[inline(never)]
  fn construct(buf: *mut Self, _args: Self::Args) {
    unsafe { ptr::write(buf, Self(Default::default())) }
  }

  #[inline(always)]
  fn get_active_scope_slot(active_scope: &mut ActiveScope) -> &mut *mut Self {
    &mut active_scope.escape_slot
  }
}
struct TryCatchData([usize; 5]);
impl ScopeData for TryCatchData {
  type Args = ();

  #[inline(never)]
  fn construct(buf: *mut Self, _args: Self::Args) {
    unsafe { ptr::write(buf, Self(Default::default())) }
  }

  #[inline(always)]
  fn get_active_scope_slot(active_scope: &mut ActiveScope) -> &mut *mut Self {
    &mut active_scope.try_catch
  }
}

#[derive(Default)]
pub struct ScopeStore {
  cookie: Cell<ScopeCookie>,
  inner: UnsafeCell<ScopeStoreInner>,
}

impl Drop for ScopeStore {
  fn drop(&mut self) {
    assert_eq!(self.cookie.get(), ScopeCookie::default());
  }
}

impl ScopeStore {
  pub fn new() -> Rc<Self> {
    Rc::new(Default::default())
  }

  #[inline(always)]
  fn with_mut<R>(
    scope: &mut impl ScopeTrait,
    f: impl Fn(&mut ScopeStoreInner) -> R,
  ) -> R {
    let scope = scope.as_mut_scope();
    let self_: &Self = &scope.store;
    assert_eq!(scope.cookie, self_.cookie.get());
    {
      let inner = unsafe { &mut *self_.inner.get() };
      debug_assert_eq!(inner.active_scope_frame_count, 0);
      inner.active_scope_frame_count = scope.frame_count;
      let result = f(inner);
      scope.frame_count = take(&mut inner.active_scope_frame_count);
      result
    }
  }

  #[inline(always)]
  fn get<D: ScopeData>(scope: &mut impl ScopeTrait) -> *mut D {
    Self::with_mut(scope, |inner| inner.get::<D>())
  }

  #[inline(always)]
  fn init_scope_with<Scope: ScopeTrait>(
    &self,
    scope: &mut Scope,
    f: impl Fn(&mut ScopeStoreInner) -> (),
  ) {
    //println!("New scope: {}", std::any::type_name::<Scope>());
    let scope = scope.as_mut_scope();

    let next_cookie = ScopeCookie::next(&self.cookie);
    ScopeCookie::set(&mut scope.cookie, next_cookie);

    debug_assert_eq!(scope.frame_count, 0);
    Self::with_mut(scope, f);
  }

  #[inline(always)]
  fn new_scope_with<'a, Scope: ScopeTrait>(
    self: &Rc<Self>,
    f: impl Fn(&mut ScopeStoreInner),
  ) -> Ref<'a, Scope> {
    let mut scope = Scope::with_store(self.clone());
    self.init_scope_with(&mut scope, f);
    Ref::<'a, Scope>::new(scope)
  }

  #[inline(always)]
  fn new_inner_scope_with<'a, Scope: ScopeTrait>(
    parent: &mut impl ScopeTrait,
    f: impl Fn(&mut ScopeStoreInner),
  ) -> Ref<'a, Scope> {
    let parent = parent.as_mut_scope();
    assert_eq!(parent.cookie, parent.store.cookie.get());
    parent.store.new_scope_with(f)
  }

  #[inline(always)]
  fn drop_scope<Scope: ScopeTrait>(scope: &mut Scope) {
    //println!("Drop scope: {}", std::any::type_name::<Scope>());
    let scope = scope.as_mut_scope();

    Self::with_mut(scope, |inner| {
      while inner.active_scope_frame_count > 0 {
        inner.pop()
      }
    });
    debug_assert_eq!(scope.frame_count, 0);

    let self_ = &scope.store;
    let cookie = ScopeCookie::revert(&self_.cookie);
    ScopeCookie::reset(&mut scope.cookie, cookie);
  }
}

struct ScopeStoreInner {
  active_scope: ActiveScope,
  frame_stack: Vec<u8>,
  active_scope_frame_count: u32,
}

impl Default for ScopeStoreInner {
  fn default() -> Self {
    Self {
      active_scope: Default::default(),
      active_scope_frame_count: 0,
      frame_stack: Vec::with_capacity(Self::FRAME_STACK_SIZE),
    }
  }
}

impl Drop for ScopeStoreInner {
  fn drop(&mut self) {
    //println!("Drop ScopeStoreInner")
    assert_eq!(self.active_scope, Default::default());
    assert_eq!(self.active_scope_frame_count, 0);
    assert_eq!(self.frame_stack.len(), 0);
  }
}

impl ScopeStoreInner {
  const FRAME_STACK_SIZE: usize = 4096 - size_of::<usize>();

  #[inline(always)]
  fn get<D: ScopeData>(&mut self) -> *mut D {
    let slot = D::get_active_scope_slot(&mut self.active_scope);
    *slot
  }

  #[inline(always)]
  fn push<D: ScopeData>(&mut self, data_args: D::Args) {
    let Self {
      active_scope,
      frame_stack,
      active_scope_frame_count,
    } = self;

    *active_scope_frame_count += 1;

    unsafe {
      // Determine byte range on the stack that the new frame will occupy.
      let frame_byte_length = size_of::<ScopeStackFrame<D>>();
      let frame_byte_offset = frame_stack.len();

      // Increase the stack limit to fit the new frame.
      let new_stack_byte_length = frame_byte_offset + frame_byte_length;
      assert!(new_stack_byte_length <= frame_stack.capacity());
      frame_stack.set_len(new_stack_byte_length);

      // Obtain a pointer to the new stack frame.
      let frame_ptr = frame_stack.get_mut(frame_byte_offset).unwrap();
      let frame_ptr: *mut ScopeStackFrame<D> = cast_mut_ptr(frame_ptr);

      // Intialize the data part of the new stack frame.
      let data_cell: &mut UnsafeCell<D> = &mut (*frame_ptr).data;
      let data_ptr = data_cell.get();
      D::construct(data_ptr, data_args);

      // Update the relevant ActiveScope pointer to point at the data that was
      // just written to the stack.
      let active_scope_slot = D::get_active_scope_slot(active_scope);
      let previous_active_ptr = replace(active_scope_slot, data_ptr);

      // Write the metadata part of the new stack frame. It contains the
      // previous value of the ActiveScope data pointer, plus a pointer to a
      // cleanup function specific to this type of frame.
      let metadata = ScopeStackFrameMetadata {
        previous_active: cast_mut_ptr(previous_active_ptr),
        cleanup_fn: Self::cleanup_frame::<D>,
      };
      let metadata_ptr: *mut _ = &mut (*frame_ptr).metadata;
      ptr::write(metadata_ptr, metadata);
    };
  }

  #[inline(always)]
  pub fn pop(&mut self) {
    let Self {
      active_scope,
      frame_stack,
      active_scope_frame_count,
    } = self;

    debug_assert!(*active_scope_frame_count > 0);
    *active_scope_frame_count -= 1;

    // Locate the metadata part of the stack frame we want to pop.
    let metadata_byte_length = size_of::<ScopeStackFrameMetadata>();
    let metadata_byte_offset = frame_stack.len() - metadata_byte_length;
    let metadata_ptr = frame_stack.get_mut(metadata_byte_offset).unwrap();
    let metadata_ptr: *mut ScopeStackFrameMetadata = cast_mut_ptr(metadata_ptr);
    let metadata = unsafe { ptr::read(metadata_ptr) };

    // Call the frame's cleanup handler.
    let cleanup_fn = metadata.cleanup_fn;
    let frame_byte_length = cleanup_fn(metadata_ptr, active_scope);
    let frame_byte_offset = frame_stack.len() - frame_byte_length;

    // Decrease the stack limit.
    unsafe { frame_stack.set_len(frame_byte_offset) };
  }

  fn cleanup_frame<D: ScopeData>(
    metadata_ptr: *mut ScopeStackFrameMetadata,
    active_scope: &mut ActiveScope,
  ) -> usize {
    // From the stack frame metadata pointer, determine the start address of the
    // whole stack frame.
    let frame_byte_length = size_of::<ScopeStackFrame<D>>();
    let metadata_byte_length = size_of::<ScopeStackFrameMetadata>();
    let byte_offset_from_frame = frame_byte_length - metadata_byte_length;
    let frame_address = (metadata_ptr as usize) - byte_offset_from_frame;
    let frame_ptr = frame_address as *mut u8;
    let frame_ptr: *mut ScopeStackFrame<D> = cast_mut_ptr(frame_ptr);
    let frame = unsafe { &mut *frame_ptr };

    // Restore the relevant ActiveScope data pointer to its previous value.
    let active_scope_slot = D::get_active_scope_slot(active_scope);
    replace(
      active_scope_slot,
      cast_mut_ptr(frame.metadata.previous_active),
    );

    // Call the destructor for the data part of the frame.
    unsafe { drop_in_place(frame.data.get()) };

    // Return the number of bytes that this frame used to occupy on the stack,
    // so `pop()` can adjust the stack limit accordingly.
    frame_byte_length
  }
}

/// Raw mutable pointer cast that checks (if necessary) that the returned
/// pointer is properly aligned.
#[inline(always)]
fn cast_mut_ptr<Source, Target>(source: *mut Source) -> *mut Target {
  let source_align = align_of::<Source>();
  let target_align = align_of::<Target>();
  let address = source as usize;
  if target_align > source_align {
    let mask = target_align - 1;
    assert_eq!(address & mask, 0);
  }
  address as *mut Target
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActiveScope {
  handle_scope: *mut HandleScopeData,
  escape_slot: *mut EscapeSlotData,
  try_catch: *mut TryCatchData,
}

impl Default for ActiveScope {
  #[inline(always)]
  fn default() -> Self {
    unsafe { MaybeUninit::zeroed().assume_init() }
  }
}
struct ScopeStackFrame<D> {
  data: UnsafeCell<D>,
  metadata: ScopeStackFrameMetadata,
}

struct ScopeStackFrameMetadata {
  previous_active: *mut (),
  cleanup_fn: fn(*mut Self, &mut ActiveScope) -> usize,
}

pub trait ScopeTrait: Sized {
  type Handles;
  type Escape;
  type TryCatch;

  fn with_store(store: Rc<ScopeStore>) -> Self;

  fn as_mut_scope(
    &mut self,
  ) -> &mut Scope<Self::Handles, Self::Escape, Self::TryCatch>;
}

impl<Handles, Escape, TryCatch> ScopeTrait
  for Scope<Handles, Escape, TryCatch>
{
  type Handles = Handles;
  type Escape = Escape;
  type TryCatch = TryCatch;

  #[inline(always)]
  fn with_store(store: Rc<ScopeStore>) -> Self {
    Self {
      store,
      cookie: ScopeCookie::INVALID,
      frame_count: 0,
      _phantom: PhantomData,
    }
  }

  #[inline(always)]
  fn as_mut_scope(&mut self) -> &mut Self {
    self
  }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
struct ScopeCookie(u32);

impl ScopeCookie {
  const INVALID: Self = Self(!0);

  #[inline(always)]
  fn next(cell: &Cell<Self>) -> Self {
    let cur_cookie = cell.get();
    assert_ne!(cur_cookie, Self::INVALID);
    let next_cookie = Self(cur_cookie.0 + 1);
    cell.set(next_cookie);
    next_cookie
  }

  #[inline(always)]
  fn revert(cell: &Cell<Self>) -> Self {
    let cur_cookie = cell.get();
    assert_ne!(cur_cookie, Self::INVALID);
    assert_ne!(cur_cookie, Self::default());
    let old_cookie = Self(cur_cookie.0 - 1);
    cell.set(old_cookie);
    cur_cookie
  }

  #[inline(always)]
  fn set(&mut self, value: Self) {
    let invalid = replace(self, value);
    assert_eq!(invalid, Self::INVALID)
  }

  #[inline(always)]
  fn reset(&mut self, value: Self) {
    let cookie = replace(self, Self::INVALID);
    assert_eq!(cookie, value);
  }
}

pub struct Yes<'t>(PhantomData<&'t ()>);
pub struct No;

pub struct Scope<Handles = No, Escape = No, TryCatch = No> {
  store: Rc<ScopeStore>,
  cookie: ScopeCookie,
  frame_count: u32,
  _phantom: PhantomData<(Handles, Escape, TryCatch)>,
}

impl<'t, Handles, Escape> Deref for Scope<Handles, Escape, Yes<'t>> {
  type Target = Scope<Handles, Escape, No>;
  #[inline(always)]
  fn deref(&self) -> &Self::Target {
    unsafe { Self::Target::cast(self) }
  }
}

impl<'t, Handles, Escape> DerefMut for Scope<Handles, Escape, Yes<'t>> {
  #[inline(always)]
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { Self::Target::cast_mut(self) }
  }
}

impl<'h, 'e> Deref for Scope<Yes<'h>, Yes<'e>, No> {
  type Target = Scope<Yes<'h>, No, No>;
  #[inline(always)]
  fn deref(&self) -> &Self::Target {
    unsafe { Self::Target::cast(self) }
  }
}

impl<'h, 'e> DerefMut for Scope<Yes<'h>, Yes<'e>, No> {
  #[inline(always)]
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { Self::Target::cast_mut(self) }
  }
}

impl<'h> Deref for Scope<Yes<'h>, No, No> {
  type Target = Scope<No, No, No>;
  #[inline(always)]
  fn deref(&self) -> &Self::Target {
    unsafe { Self::Target::cast(self) }
  }
}

impl<'h> DerefMut for Scope<Yes<'h>, No, No> {
  #[inline(always)]
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { Self::Target::cast_mut(self) }
  }
}

impl<Handles, Escape, TryCatch> Scope<Handles, Escape, TryCatch> {
  #[inline(always)]
  unsafe fn cast<Handles_, Escape_, TryCatch_>(
    from: &Scope<Handles_, Escape_, TryCatch_>,
  ) -> &Self {
    &*(from as *const _ as *const Self)
  }

  #[inline(always)]
  unsafe fn cast_mut<Handles_, Escape_, TryCatch_>(
    from: &mut Scope<Handles_, Escape_, TryCatch_>,
  ) -> &mut Self {
    &mut *(from as *mut _ as *mut Self)
  }
}

impl Scope<No, No, No> {
  fn root<'a>(store: &'_ Rc<ScopeStore>) -> Ref<'a, Self> {
    store.new_scope_with(|_| ())
  }
}

impl<'h, Escape, TryCatch> Scope<Yes<'h>, Escape, TryCatch> {
  pub fn handle_scope<'a, Handles_>(
    parent: &'a mut Scope<Handles_, Escape, TryCatch>,
  ) -> Ref<'a, Self> {
    ScopeStore::new_inner_scope_with(parent, |s| {
      s.push::<HandleScopeData>(());
    })
  }

  pub fn make_local<T>(&'_ mut self) -> Local<'h, T> {
    // Do not remove. This access verifies that `self` is the topmost scope.
    let _: *mut HandleScopeData = ScopeStore::get(self);
    Default::default()
  }
}

impl<'h, 'e: 'h, TryCatch> Scope<Yes<'h>, Yes<'e>, TryCatch> {
  pub fn escapable_handle_scope<'a, Escape_>(
    parent: &'a mut Scope<Yes<'e>, Escape_, TryCatch>,
  ) -> Ref<'a, Self> {
    ScopeStore::new_inner_scope_with(parent, |s| {
      s.push::<EscapeSlotData>(());
      s.push::<HandleScopeData>(());
    })
  }

  pub fn escape<T>(&'_ mut self, local: Local<'h, T>) -> Local<'e, T> {
    let escape_slot_ptr: *mut EscapeSlotData = ScopeStore::get(self);
    assert!(size_of_val(&local) <= size_of::<EscapeSlotData>());
    let local_in_ptr = escape_slot_ptr as *mut Local<'h, T>;
    unsafe { ptr::write(local_in_ptr, local) };
    let local_out_ptr = escape_slot_ptr as *mut Local<'e, T>;
    unsafe { ptr::read(local_out_ptr) }
  }
}

impl<'t, Handles, Escape> Scope<Handles, Escape, Yes<'t>> {
  pub fn try_catch<'a, TryCatch_>(
    parent: &'a mut Scope<Handles, Escape, TryCatch_>,
  ) -> Ref<'a, Self> {
    ScopeStore::new_inner_scope_with(parent, |s| {
      s.push::<TryCatchData>(());
    })
  }
}

pub type HandleScope<'h> = Scope<Yes<'h>, No, No>;

impl<'h> HandleScope<'h> {
  #[allow(clippy::new_ret_no_self)]
  pub fn new<'a, Handles_, Escape, TryCatch>(
    parent: &'a mut Scope<Handles_, Escape, TryCatch>,
  ) -> Ref<'a, Scope<Yes<'h>, Escape, TryCatch>> {
    Scope::handle_scope(parent)
  }
}

pub type EscapableHandleScope<'h, 'e> = Scope<Yes<'h>, Yes<'e>, No>;

impl<'h, 'e: 'h> EscapableHandleScope<'h, 'e> {
  #[allow(clippy::new_ret_no_self)]
  pub fn new<'a, Escape_, TryCatch>(
    parent: &'a mut Scope<Yes<'e>, Escape_, TryCatch>,
  ) -> Ref<'a, Scope<Yes<'h>, Yes<'e>, TryCatch>> {
    Scope::escapable_handle_scope(parent)
  }
}

pub type TryCatch<'t> = Scope<No, No, Yes<'t>>;

impl<'t> TryCatch<'t> {
  #[allow(clippy::new_ret_no_self)]
  pub fn new<'a, Handles, Escape, TryCatch_>(
    parent: &'a mut Scope<Handles, Escape, TryCatch_>,
  ) -> Ref<'a, Scope<Handles, Escape, Yes<'t>>> {
    Scope::try_catch(parent)
  }
}

pub struct Ref<'a, Scope: ScopeTrait> {
  scope: Scope,
  _lifetime: PhantomData<&'a mut ()>,
}

impl<'a, Scope: ScopeTrait> Ref<'a, Scope> {
  #[inline(always)]
  fn new(scope: Scope) -> Self {
    Self {
      scope,
      _lifetime: PhantomData,
    }
  }
}

impl<'a, Scope: ScopeTrait> Drop for Ref<'a, Scope> {
  #[inline(always)]
  fn drop(&mut self) {
    ScopeStore::drop_scope(&mut self.scope)
  }
}

impl<'a, Scope: ScopeTrait> Deref for Ref<'a, Scope> {
  type Target = Scope;
  #[inline(always)]
  fn deref(&self) -> &Self::Target {
    &self.scope
  }
}

impl<'a, Scope: ScopeTrait> DerefMut for Ref<'a, Scope> {
  #[inline(always)]
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.scope
  }
}

#[derive(Copy, Clone)]
struct Value(*mut ());

#[derive(Copy, Clone)]
struct Context(*mut ());

#[derive(Copy, Clone)]
pub struct Local<'a, T> {
  _phantom: PhantomData<&'a T>,
  _ptr: *mut T,
}

impl<'a, T> Default for Local<'a, T> {
  fn default() -> Self {
    Local {
      _phantom: PhantomData,
      _ptr: null_mut(),
    }
  }
}

struct Global<T> {
  _phantom: PhantomData<T>,
  _ptr: *mut T,
}

impl<T> Global<T> {
  fn new() -> Self {
    Self {
      _phantom: PhantomData,
      _ptr: null_mut(),
    }
  }
}

impl<T> Deref for Global<T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    unsafe { &*self._ptr }
  }
}

impl<'h, T> Local<'h, T> {
  fn new<'a, Escape, TryCatch>(
    scope: &'a mut Scope<Yes<'h>, Escape, TryCatch>,
  ) -> Self
  where
    'h: 'a,
  {
    scope.make_local::<T>()
  }
}

impl<'a, T> Deref for Local<'a, T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    unsafe { &*self._ptr }
  }
}

fn indirect_make_local<'h, T, Escape, TryCatch>(
  scope: &'_ mut Scope<Yes<'h>, Escape, TryCatch>,
) -> Local<'h, T> {
  Local::new(scope)
}

#[inline(never)]
fn use_it<T>(_: &T) {}

fn use_local<T>(_: &T) {}

struct Stuff<'a>(&'a Value, &'a Value, &'a Value);

fn create_local_in_handle_scope<'a>(
  scope: &mut HandleScope<'a>,
) -> Local<'a, Value> {
  Local::<Value>::new(scope)
}

fn create_local_in_escapable_handle_scope<'h, 'e>(
  scope: &mut EscapableHandleScope<'h, 'e>,
) -> Local<'h, Value> {
  Local::<Value>::new(scope)
}

#[allow(unused_variables)]
fn testing() {
  let store = ScopeStore::new();
  let root = &mut Scope::root(&store);
  let hs = &mut Scope::handle_scope(root);
  let esc1 = &mut Scope::escapable_handle_scope(hs);
  let esc2 = &mut EscapableHandleScope::new(esc1);
  let ehs = &mut Scope::handle_scope(esc2);
  let l1 = ehs.make_local::<Value>();
  let e1 = ehs.escape(l1);
  let tc = &mut TryCatch::new(ehs);
  create_local_in_escapable_handle_scope(tc);
  let tcl1 = Local::<Value>::new(tc);
  let e1 = tc.escape(l1);
  let e1 = tc.escape(l1);
  let hs = &mut Scope::handle_scope(tc);
}

fn main() {
  testing();

  let store1 = ScopeStore::new();
  let root1 = &mut Scope::root(&store1);
  let store2 = ScopeStore::new();
  let root2 = &mut Scope::root(&store2);
  {
    let x = &mut Scope::handle_scope(root1);
    let _xxv = x.make_local::<Value>();
    let yyv = {
      let mut y = HandleScope::new(x);
      //std::mem::swap(&mut x, &mut y);
      //let r1 = Local::<Value>::new(x);
      //let r2 = (y.get_make_local())();
      let r1 = y.make_local::<Value>();
      let r2 = y.make_local::<Value>();
      let r3 = Local::<Value>::new(&mut y);
      {
        let sc = &mut Scope::root(&store1);
        let sc: &mut Ref<_> = &mut Scope::handle_scope(sc);
        //let _panic = Local::<Value>::new(&mut y);
        let _scl = Local::<Value>::new(sc);
      }
      use_local(&r3);
      let r4 = Local::<Value>::new(&mut y);
      use_local(&r3);
      let g = Some(Global::<Value>::new());
      let stuff = Stuff(&r1, &r2, g.as_ref().unwrap());
      //g.replace(Global::new());
      use_local(&r1);
      use_local(g.as_ref().unwrap());
      use_it(&stuff);
      let _r5: Local<Value> = indirect_make_local(&mut y);
      let z1 = {
        let w0 = &mut Scope::handle_scope(&mut y);
        let wl0 = Local::<Value>::new(w0);
        {
          let w1 = &mut Scope::handle_scope(w0);
          let _wl1 = Local::<Value>::new(w1);
          let tc = &mut Scope::try_catch(w1);
          let _tcl1 = create_local_in_handle_scope(tc);
        }
        let w2 = &mut HandleScope::new(w0);
        //let wl0x = Local::<Value>::new(w0);
        let _wl2 = Local::<Value>::new(w2);
        use_it(&r1);
        use_it(&r2);
        use_it(&r3);
        use_it(&r4);
        wl0
      };
      use_it(&z1);
      let mut y2 = Scope::handle_scope(&mut y);
      //u = y2;
      //r
      //use_it(&z1);
      //use_it(&r5);
      //std::mem::swap(y2, y);
      let z2 = Local::<Value>::new(&mut y2);
      let _z3 = Scope::handle_scope(&mut y2);
      use_it(&r4);
      use_it(&z2);
      //_z3
      1
    };
    let _y2 = Scope::handle_scope(root2);
    //drop(root2);
    //use_it(&xxv);
    //drop(x);
    let mut _q = HandleScope::new(x);
    use_it(&yyv);
    //use_it(u);
  }

  //let mut xb: Scope = Scope::new(&mut x);
  //let mut a = Scope::root();
  //let mut b1 = Scope::new(&mut a);
  //let v1 = Local::new(&mut b1);
  ////std::mem::swap(&mut xb, &mut b1);
  ////let xc = Scope::new(&mut b1);
  //let v2 = Local::new(&mut b1);
  //let mut c = Scope::new(&mut b1);
  ////drop(b1);
  ////drop(b1);
  //drop(v1);
  //println!("Hello, world!");
}

pub fn godbolt() {
  let store = ScopeStore::new();
  let ref mut root = Scope::root(&store);
  {
    let s1 = &mut HandleScope::new(root);
    let mut l1a = Local::<Value>::new(s1);
    let _l1b = l1a;
    {
      let s2 = &mut EscapableHandleScope::new(s1);
      let l2a = Local::<Value>::new(s2);
      let _l2b = Local::<Value>::new(s2);
      l1a = s2.escape(l2a);
      use_it(&l1a);
    }
  }
}
