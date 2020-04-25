use std::any::Any;
use std::cell::Cell;
use std::cell::RefCell;
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::replace;
use std::mem::size_of;
use std::mem::size_of_val;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr;
use std::ptr::drop_in_place;
use std::ptr::null_mut;
use std::rc::Rc;

#[derive(Clone, Copy)]
pub struct ScopeTop {
  handle_scope: *mut HandleScopeData,
  escape_slot: *mut EscapeSlotData,
  try_catch: *mut TryCatchData,
}

impl Default for ScopeTop {
  fn default() -> Self {
    unsafe { MaybeUninit::zeroed().assume_init() }
  }
}

#[derive(Default)]
pub struct ScopeManager {
  cookie: Cell<u32>,
  inner: RefCell<ScopeManagerInner>,
}

impl Drop for ScopeManager {
  fn drop(&mut self) {
    assert_eq!(self.cookie.get(), 0);
  }
}

impl ScopeManager {
  pub fn new() -> Rc<Self> {
    Rc::new(Default::default())
  }

  #[allow(clippy::mut_from_ref)]
  fn get(&self, cookie: u32) -> &RefCell<ScopeManagerInner> {
    assert_eq!(cookie, self.cookie.get());
    &self.inner
  }

  fn new_root(&self) -> u32 {
    let cookie = self.cookie.get() + 1;
    self.cookie.set(cookie);
    cookie
  }

  fn shadow(&self, mut cookie: u32) -> u32 {
    assert_eq!(cookie, self.cookie.get());
    cookie += 1;
    self.cookie.set(cookie);
    cookie
  }

  fn unshadow(&self, cookie: u32) {
    assert_eq!(cookie, self.cookie.get());
    self.cookie.set(cookie - 1);
  }
}

pub struct ScopeManagerInner {
  top: ScopeTop,
  stack: Vec<u8>,
}

impl Default for ScopeManagerInner {
  fn default() -> Self {
    Self {
      top: Default::default(),
      stack: Vec::with_capacity(Self::SCOPE_STACK_SIZE),
    }
  }
}

impl Drop for ScopeManagerInner {
  fn drop(&mut self) {
    assert_eq!(self.stack.len(), 0);
  }
}

impl ScopeManagerInner {
  const SCOPE_STACK_SIZE: usize = 4096 - size_of::<usize>();

  pub fn push<D: ScopeStackItemData>(&mut self, args: D::Args) -> *mut D {
    let scope_stack = &mut self.stack;
    let frame_byte_length = size_of::<ScopeStackItemFrame<D>>();
    let stack_byte_offset = scope_stack.len();
    let new_stack_byte_length = stack_byte_offset + frame_byte_length;
    assert!(new_stack_byte_length <= scope_stack.capacity());
    unsafe { scope_stack.set_len(new_stack_byte_length) };

    let frame = unsafe {
      let frame_ptr = scope_stack.get_mut(stack_byte_offset).unwrap() as *mut u8
        as *mut ScopeStackItemFrame<D>;
      let data_cell: &mut UnsafeCell<D> = &mut (*frame_ptr).data;
      let data_ptr = data_cell.get();
      let meta_ptr: *mut _ = &mut (*frame_ptr).meta;

      D::construct(data_ptr, args);

      let meta = ScopeStackItemMeta {
        previous_top: replace(D::get_top_slot(&mut self.top), data_ptr)
          as *mut (),
        cleanup_fn: Self::cleanup_frame::<D>,
      };
      ptr::write(meta_ptr, meta);

      &mut *frame_ptr
    };
    frame.data.get()
  }

  pub fn pop(&mut self) {
    let scope_stack = &mut self.stack;
    let meta_byte_length = size_of::<ScopeStackItemMeta>();
    let meta_byte_offset = scope_stack.len() - meta_byte_length;
    let meta_ptr = scope_stack.get_mut(meta_byte_offset).unwrap() as *mut u8
      as *mut ScopeStackItemMeta;
    let meta = unsafe { ptr::read(meta_ptr) };
    let cleanup_fn = meta.cleanup_fn;
    let frame_byte_length = cleanup_fn(meta_ptr, &mut self.top);
    let frame_byte_offset = scope_stack.len() - frame_byte_length;
    unsafe { scope_stack.set_len(frame_byte_offset) };
  }

  fn cleanup_frame<D: ScopeStackItemData>(
    meta_ptr: *mut ScopeStackItemMeta,
    top: &mut ScopeTop,
  ) -> usize {
    let frame_byte_length = size_of::<ScopeStackItemFrame<D>>();
    let meta_byte_length = size_of::<ScopeStackItemMeta>();
    let byte_offset_from_frame = frame_byte_length - meta_byte_length;
    let frame_ptr =
      unsafe { (meta_ptr as *mut u8).sub(byte_offset_from_frame) };
    let frame_ptr = frame_ptr as *mut ScopeStackItemFrame<D>;
    let frame = unsafe { &mut *frame_ptr };
    replace(D::get_top_slot(top), frame.meta.previous_top as *mut D);
    unsafe { drop_in_place(frame.data.get()) };
    frame_byte_length
  }
}

pub trait ScopeStackItemData {
  type Args;
  fn construct(buf: *mut Self, args: Self::Args);
  fn get_top_slot(top: &mut ScopeTop) -> &mut *mut Self;
}

struct ScopeStackItemFrame<D> {
  data: UnsafeCell<D>,
  meta: ScopeStackItemMeta,
}

struct ScopeStackItemMeta {
  previous_top: *mut (),
  cleanup_fn: fn(*mut Self, &mut ScopeTop) -> usize,
}

struct For<'t>(PhantomData<&'t ()>);
struct Never;
//type Never = std::convert::Infallible; // Forward compatible.

struct Scope<Handles = Never, Escape = Never, TryCatch = Never> {
  mgr: Rc<ScopeManager>,
  cookie: u32,
  frames: u32,
  _phantom: PhantomData<(Handles, Escape, TryCatch)>,
}

impl<'t, Handles, Escape> Deref for Scope<Handles, Escape, For<'t>> {
  type Target = Scope<Handles, Escape, Never>;
  fn deref(&self) -> &Self::Target {
    unsafe { Self::Target::cast(self) }
  }
}

impl<'t, Handles, Escape> DerefMut for Scope<Handles, Escape, For<'t>> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { Self::Target::cast_mut(self) }
  }
}

impl<'l, 'e> Deref for Scope<For<'l>, For<'e>, Never> {
  type Target = Scope<For<'l>, Never, Never>;
  fn deref(&self) -> &Self::Target {
    unsafe { Self::Target::cast(self) }
  }
}

impl<'l, 'e> DerefMut for Scope<For<'l>, For<'e>, Never> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { Self::Target::cast_mut(self) }
  }
}

impl<'l> Deref for Scope<For<'l>, Never, Never> {
  type Target = Scope<Never, Never, Never>;
  fn deref(&self) -> &Self::Target {
    unsafe { Self::Target::cast(self) }
  }
}

impl<'l> DerefMut for Scope<For<'l>, Never, Never> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { Self::Target::cast_mut(self) }
  }
}

impl<Handles, Escape, TryCatch> Scope<Handles, Escape, TryCatch> {
  pub fn _dup<'a>(
    parent: &'a mut Scope<Handles, Escape, TryCatch>,
  ) -> ScopeRef<'a, Handles, Escape, TryCatch> {
    let mgr = parent.mgr.clone();
    let cookie = mgr.shadow(parent.cookie);
    let self_ = Self {
      cookie,
      mgr,
      frames: 0,
      _phantom: PhantomData,
    };
    ScopeRef::new(self_)
  }

  unsafe fn cast<Handles_, Escape_, TryCatch_>(
    from: &Scope<Handles_, Escape_, TryCatch_>,
  ) -> &Self {
    &*(from as *const _ as *const Self)
  }

  unsafe fn cast_mut<Handles_, Escape_, TryCatch_>(
    from: &mut Scope<Handles_, Escape_, TryCatch_>,
  ) -> &mut Self {
    &mut *(from as *mut _ as *mut Self)
  }
}

impl Scope<Never, Never, Never> {
  pub fn root<'a>(
    mgr: &'_ Rc<ScopeManager>,
  ) -> ScopeRef<'a, Never, Never, Never> {
    let mgr = mgr.clone();
    let cookie = mgr.new_root();
    let self_ = Self {
      mgr,
      cookie,
      frames: 0,
      _phantom: PhantomData,
    };
    ScopeRef::new(self_)
  }
}

impl<'l, Escape, TryCatch> Scope<For<'l>, Escape, TryCatch> {
  pub fn with_handles<'a, Handles_>(
    parent: &'a mut Scope<Handles_, Escape, TryCatch>,
  ) -> ScopeRef<'a, For<'l>, Escape, TryCatch> {
    let mgr = parent.mgr.clone();
    let cookie = mgr.shadow(parent.cookie);
    mgr.get(cookie).borrow_mut().push::<HandleScopeData>(());
    let self_ = Scope {
      cookie,
      mgr,
      frames: 1,
      _phantom: PhantomData,
    };
    ScopeRef::new(self_)
  }

  pub fn make_local<T>(&'_ mut self) -> Local<'l, T> {
    let _ = self.mgr.get(self.cookie); // Just check cookie.
    Default::default()
  }
}

impl<'e, TryCatch> Scope<For<'e>, For<'e>, TryCatch> {
  #[allow(dead_code)]
  pub fn with_escape<'a, Escape_>(
    parent: &'a mut Scope<For<'e>, Escape_, TryCatch>,
  ) -> ScopeRef<'a, For<'e>, For<'e>, TryCatch> {
    let mgr = parent.mgr.clone();
    let cookie = mgr.shadow(parent.cookie);
    mgr.get(cookie).borrow_mut().push::<EscapeSlotData>(());
    let self_ = Scope {
      cookie,
      mgr,
      frames: 1,
      _phantom: PhantomData,
    };
    ScopeRef::new(self_)
  }

  pub fn escape<'l, T>(&'_ mut self, local: Local<'l, T>) -> Local<'e, T> {
    let escape_slot_ptr =
      self.mgr.get(self.cookie).borrow_mut().top.escape_slot;
    assert!(size_of_val(&local) <= size_of::<EscapeSlotData>());
    let local_in_ptr = escape_slot_ptr as *mut Local<'l, T>;
    unsafe { ptr::write(local_in_ptr, local) };
    let local_out_ptr = escape_slot_ptr as *mut Local<'e, T>;
    unsafe { ptr::read(local_out_ptr) }
  }
}

struct ScopeRef<'a, Handles, Escape, TryCatch> {
  scope: Scope<Handles, Escape, TryCatch>,
  _lifetime: PhantomData<&'a mut ()>,
}

impl<'a, Handles, Escape, TryCatch> ScopeRef<'a, Handles, Escape, TryCatch> {
  fn new(scope: Scope<Handles, Escape, TryCatch>) -> Self {
    println!("New scope: {}", std::any::type_name::<Self>());
    Self {
      scope,
      _lifetime: PhantomData,
    }
  }
}

impl<'a, Handles, Escape, TryCatch> Drop
  for ScopeRef<'a, Handles, Escape, TryCatch>
{
  fn drop(&mut self) {
    println!("Drop scope: {}", std::any::type_name::<Self>());
    for _ in 0..self.frames {
      self.mgr.get(self.cookie).borrow_mut().pop()
    }
    self.mgr.unshadow(self.cookie)
  }
}

impl<'a, Handles, Escape, TryCatch> Deref
  for ScopeRef<'a, Handles, Escape, TryCatch>
{
  type Target = Scope<Handles, Escape, TryCatch>;
  fn deref(&self) -> &Self::Target {
    &self.scope
  }
}

impl<'a, Handles, Escape, TryCatch> DerefMut
  for ScopeRef<'a, Handles, Escape, TryCatch>
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.scope
  }
}

pub trait AsScopeRef<'a> {
  type ScopeRef;
}
impl<'a, Handles, Escape, TryCatch> AsScopeRef<'a>
  for Scope<Handles, Escape, TryCatch>
{
  type ScopeRef = ScopeRef<'a, Handles, Escape, TryCatch>;
}

trait NewScope<'a, Input>: AsScopeRef<'a> {
  fn new(parent: &'a mut Input) -> Self::ScopeRef;
}

type HandleScope<'l> = Scope<For<'l>, Never, Never>;

impl<'a, 'l, Handles_, Escape_, TryCatch>
  NewScope<'a, Scope<Handles_, Escape_, TryCatch>> for HandleScope<'l>
{
  fn new(parent: &'a mut Scope<Handles_, Escape_, TryCatch>) -> Self::ScopeRef {
    let mgr = parent.mgr.clone();
    let cookie = mgr.shadow(parent.cookie);
    mgr.get(cookie).borrow_mut().push::<HandleScopeData>(());
    let self_ = Scope {
      cookie,
      mgr,
      frames: 1,
      _phantom: PhantomData,
    };
    ScopeRef::new(self_)
  }
}

type EscapableHandleScope<'l, 'e> = Scope<For<'l>, For<'e>, Never>;

impl<'a, 'l: 'e, 'e, Escape_, TryCatch_>
  NewScope<'a, Scope<For<'e>, Escape_, TryCatch_>>
  for EscapableHandleScope<'l, 'e>
{
  fn new(parent: &'a mut Scope<For<'e>, Escape_, TryCatch_>) -> Self::ScopeRef {
    let mgr = parent.mgr.clone();
    let cookie = mgr.shadow(parent.cookie);
    mgr.get(cookie).borrow_mut().push::<EscapeSlotData>(());
    mgr.get(cookie).borrow_mut().push::<HandleScopeData>(());
    let self_ = Scope {
      cookie,
      mgr,
      frames: 2,
      _phantom: PhantomData,
    };
    ScopeRef::new(self_)
  }
}

type TryCatch<'t, 'l, 'e, Handles, Escape> = Scope<Handles, Escape, For<'t>>;

impl<'a, 't, 'l, 'e, Handles, Escape, TryCatch_>
  NewScope<'a, Scope<Handles, Escape, TryCatch_>>
  for TryCatch<'t, 'l, 'e, Handles, Escape>
{
  fn new(parent: &'a mut Scope<Handles, Escape, TryCatch_>) -> Self::ScopeRef {
    let mgr = parent.mgr.clone();
    let cookie = mgr.shadow(parent.cookie);
    mgr.get(cookie).borrow_mut().push::<TryCatchData>(());
    let self_ = Scope {
      cookie,
      mgr,
      frames: 1,
      _phantom: PhantomData,
    };
    ScopeRef::new(self_)
  }
}

struct HandleScopeData([usize; 3]);
impl ScopeStackItemData for HandleScopeData {
  type Args = ();
  fn construct(buf: *mut Self, _args: Self::Args) {
    unsafe { ptr::write(buf, Self(Default::default())) }
  }
  fn get_top_slot(top: &mut ScopeTop) -> &mut *mut Self {
    &mut top.handle_scope
  }
}

struct EscapeSlotData([usize; 1]);
impl ScopeStackItemData for EscapeSlotData {
  type Args = ();
  fn construct(buf: *mut Self, _args: Self::Args) {
    unsafe { ptr::write(buf, Self(Default::default())) }
  }
  fn get_top_slot(top: &mut ScopeTop) -> &mut *mut Self {
    &mut top.escape_slot
  }
}
struct TryCatchData([usize; 5]);
impl ScopeStackItemData for TryCatchData {
  type Args = ();
  fn construct(buf: *mut Self, _args: Self::Args) {
    unsafe { ptr::write(buf, Self(Default::default())) }
  }
  fn get_top_slot(top: &mut ScopeTop) -> &mut *mut Self {
    &mut top.try_catch
  }
}

#[derive(Copy, Clone)]
struct Value(*mut ());

#[derive(Copy, Clone)]
struct Local<'a, T> {
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

impl<'l, T> Local<'l, T> {
  fn new<'a, Escape, TryCatch>(
    scope: &'a mut Scope<For<'l>, Escape, TryCatch>,
  ) -> Self
  where
    'l: 'a,
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

fn indirect_make_local<'l, T, Escape, TryCatch>(
  scope: &'_ mut Scope<For<'l>, Escape, TryCatch>,
) -> Local<'l, T> {
  Local::new(scope)
}

fn use_it<T>(_: &T) {}

fn use_local<T>(_: &T) {}

struct Stuff<'a>(&'a Value, &'a Value, &'a Value);

fn call_with_try_catch(_tc: &mut TryCatch<impl Any, impl Any>) {}

fn create_local_in_handle_scope<'a>(
  scope: &mut HandleScope<'a>,
) -> Local<'a, Value> {
  Local::<Value>::new(scope)
}

#[allow(unused_variables)]
pub fn testing() {
  let mgr = ScopeManager::new();
  let root = &mut Scope::root(&mgr);
  let hs = &mut Scope::with_handles(root);
  let esc = &mut EscapableHandleScope::new(hs);
  let ehs = &mut Scope::with_handles(esc);
  let l1 = ehs.make_local::<Value>();
  let e1 = ehs.escape(l1);
  let tc = &mut TryCatch::new(ehs);
  call_with_try_catch(tc);
  let tcl1 = Local::<Value>::new(tc);
  let e1 = tc.escape(l1);
  let e1 = tc.escape(l1);
  let hs = &mut HandleScope::new(tc);
}

fn main() {
  testing();

  let mgr1 = ScopeManager::new();
  let root1 = &mut Scope::root(&mgr1);
  let mgr2 = ScopeManager::new();
  let root2 = &mut Scope::root(&mgr2);
  {
    let x = &mut Scope::with_handles(root1);
    let _xxv = x.make_local::<Value>();
    let yyv = {
      let mut y = Scope::with_handles(x);
      //std::mem::swap(&mut x, &mut y);
      //let r1 = Local::<Value>::new(x);
      //let r2 = (y.get_make_local())();
      let r1 = y.make_local::<Value>();
      let r2 = y.make_local::<Value>();
      let r3 = Local::<Value>::new(&mut y);
      {
        let sc = &mut Scope::root(&mgr1);
        let sc = &mut Scope::with_handles(sc);
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
        let w0 = &mut Scope::with_handles(&mut y);
        let wl0 = Local::<Value>::new(w0);
        {
          let w1 = &mut Scope::with_handles(w0);
          let _wl1 = Local::<Value>::new(w1);
          let tc = &mut TryCatch::new(w1);
          let _tcl1 = create_local_in_handle_scope(tc);
        }
        let w2 = &mut Scope::with_handles(w0);
        //let wl0x = Local::<Value>::new(w0);
        let _wl2 = Local::<Value>::new(w2);
        use_it(&r1);
        use_it(&r2);
        use_it(&r3);
        use_it(&r4);
        wl0
      };
      use_it(&z1);
      let ref mut y2 = HandleScope::new(&mut y);
      //u = y2;
      //r
      //use_it(&z1);
      //use_it(&r5);
      //std::mem::swap(y2, y);
      let z2 = Local::<Value>::new(y2);
      let _z3 = Scope::with_handles(y2);
      use_it(&r4);
      use_it(&z2);
    };
    let _y2 = Scope::with_handles(root2);
    //drop(root2);
    //use_it(&xxv);
    //drop(x);
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
