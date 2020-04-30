use std::cell::Cell;
use std::marker::PhantomData;
use std::mem::align_of;
use std::mem::needs_drop;
use std::mem::replace;
use std::mem::size_of;
use std::mem::take;
use std::mem::transmute;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr;
use std::ptr::drop_in_place;
use std::ptr::null;
use std::ptr::null_mut;
use std::ptr::NonNull;
use std::rc::Rc;

pub(crate) use internal::ScopeStore;

use internal::ActiveScopeData;
use internal::ScopeCookie;
use internal::ScopeData;
use params::ScopeParams;
use params::{No, Yes};

pub struct Ref<'a, Scope: ScopeParams> {
  scope: Scope,
  _lifetime: PhantomData<&'a mut ()>,
}

impl<'a, Scope: ScopeParams> Ref<'a, Scope> {
  #[inline(always)]
  fn new(scope: Scope) -> Self {
    Self {
      scope,
      _lifetime: PhantomData,
    }
  }
}

impl<'a, Scope: ScopeParams> Drop for Ref<'a, Scope> {
  #[inline(always)]
  fn drop(&mut self) {
    ScopeStore::drop_scope(&mut self.scope)
  }
}

impl<'a, Scope: ScopeParams> Deref for Ref<'a, Scope> {
  type Target = Scope;
  #[inline(always)]
  fn deref(&self) -> &Self::Target {
    &self.scope
  }
}

impl<'a, Scope: ScopeParams> DerefMut for Ref<'a, Scope> {
  #[inline(always)]
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.scope
  }
}

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
  pub fn from_isolate<'a>(isolate: &'_ Isolate) -> Ref<'a, Self> {
    let scope_store = isolate.get_scopes();
    ScopeStore::new_scope_with(scope_store, |s| {
      s.assert_same_isolate(isolate);
      s.push::<data::Context>(None);
    })
  }

  pub fn from_context<'a>(
    context: impl Deref<Target = Context>,
  ) -> Ref<'a, Self> {
    let context_ptr: *const Context = &*context;
    let context_ptr = NonNull::new(context_ptr as *mut _).unwrap();
    let isolate = context.get_isolate();
    let scope_store = isolate.get_scopes();
    ScopeStore::new_scope_with(scope_store, |s| {
      s.assert_same_isolate(isolate);
      s.push::<data::Context>(Some(context_ptr));
    })
  }
}

impl<Handles, Escape, TryCatch> Scope<Handles, Escape, TryCatch> {
  pub fn context_scope<'a>(
    parent: &'a mut Scope<Handles, Escape, TryCatch>,
    context: impl Deref<Target = Context>,
  ) -> Ref<'a, Self> {
    let context_ptr: *const Context = &*context;
    let context_ptr = NonNull::new(context_ptr as *mut _).unwrap();
    ScopeStore::new_inner_scope_with(parent, |s| {
      s.push::<data::Context>(Some(context_ptr));
    })
  }
}

impl<'h, Escape, TryCatch> Scope<Yes<'h>, Escape, TryCatch> {
  pub fn handle_scope<'a, Handles_>(
    parent: &'a mut Scope<Handles_, Escape, TryCatch>,
  ) -> Ref<'a, Self> {
    ScopeStore::new_inner_scope_with(parent, |s| {
      s.push::<data::HandleScope>(());
    })
  }

  #[inline(always)]
  pub fn to_local<T>(&'_ mut self, ptr: *const T) -> Local<'h, T> {
    // Do not remove. This access verifies that `self` is the topmost scope.
    let _: data::HandleScope = ScopeStore::get(self);
    Local::from_raw(ptr)
  }
}

impl<'h, 'e: 'h, TryCatch> Scope<Yes<'h>, Yes<'e>, TryCatch> {
  pub fn escapable_handle_scope<'a, Escape_>(
    parent: &'a mut Scope<Yes<'e>, Escape_, TryCatch>,
  ) -> Ref<'a, Self> {
    ScopeStore::new_inner_scope_with(parent, |s| {
      s.push::<data::EscapeSlot>(());
      s.push::<data::HandleScope>(());
    })
  }

  pub fn escape<T: Copy>(&'_ mut self, local: Local<'h, T>) -> Local<'e, T> {
    let escape_slot: data::EscapeSlot = ScopeStore::take(self);
    let mut value_slot = match *escape_slot {
      None => panic!("only one value can escape from an EscapableHandleScope"),
      Some(p) => p,
    };
    let value_slot: &mut *const _ = unsafe { value_slot.as_mut() };
    *value_slot = unsafe { transmute(local) };
    unsafe { transmute(local) }
  }
}

impl<'t, Handles, Escape> Scope<Handles, Escape, Yes<'t>> {
  pub fn try_catch<'a, TryCatch_>(
    parent: &'a mut Scope<Handles, Escape, TryCatch_>,
  ) -> Ref<'a, Self> {
    ScopeStore::new_inner_scope_with(parent, |s| {
      s.push::<data::TryCatch>(());
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

mod params {
  use super::*;

  pub struct Yes<'t>(PhantomData<&'t ()>);
  pub struct No;

  pub trait ScopeParams: Sized {
    type Handles;
    type Escape;
    type TryCatch;

    fn as_scope_mut(
      &mut self,
    ) -> &mut Scope<Self::Handles, Self::Escape, Self::TryCatch>;
  }

  impl<Handles, Escape, TryCatch> ScopeParams
    for Scope<Handles, Escape, TryCatch>
  {
    type Handles = Handles;
    type Escape = Escape;
    type TryCatch = TryCatch;

    #[inline(always)]
    fn as_scope_mut(&mut self) -> &mut Self {
      self
    }
  }
}

mod data {
  use super::*;

  #[derive(Clone, Copy)]
  pub(super) enum Context {
    Current,
    CurrentCached(Option<NonNull<super::Context>>),
    Entered(NonNull<super::Context>),
  }

  impl Default for Context {
    fn default() -> Self {
      Self::Current
    }
  }

  impl ScopeData for Context {
    type Args = Option<NonNull<super::Context>>;
    type Raw = ();

    #[inline(always)]
    fn activate(
      _raw: *mut Self::Raw,
      args: &mut Self::Args,
      _isolate: &mut Isolate,
      active_scope_data: &mut ActiveScopeData,
    ) -> Self {
      let active = match args.take() {
        None => Self::default(),
        Some(handle) => Self::Entered(handle),
      };
      replace(&mut active_scope_data.context, active)
      // TODO: enter if entered.
    }

    #[inline(always)]
    fn deactivate(
      _raw: *mut Self::Raw,
      previous: Self,
      _isolate: &mut Isolate,
      active_scope_data: &mut ActiveScopeData,
    ) {
      // TODO: exit if entered.
      replace(&mut active_scope_data.context, previous);
    }

    #[inline(always)]
    fn get_mut<'a>(
      isolate: &'a mut Isolate,
      active_scope_data: &'a mut ActiveScopeData,
    ) -> &'a mut Self {
      if let Self::Current = active_scope_data.context {
        let current_context = isolate
          .get_current_context()
          .map(|local| -> *const super::Context { &*local })
          .map(|ptr| ptr as *mut _)
          .and_then(NonNull::new);
        replace(
          &mut active_scope_data.context,
          Self::CurrentCached(current_context),
        );
      }
      &mut active_scope_data.context
    }
  }

  #[derive(Clone, Copy, Default)]
  pub(super) struct HandleScope(Option<NonNull<<Self as ScopeData>::Raw>>);

  impl ScopeData for HandleScope {
    type Args = ();
    type Raw = [usize; 3];

    #[inline(always)]
    fn construct(
      buf: *mut Self::Raw,
      _args: &mut Self::Args,
      _isolate: &mut Isolate,
    ) {
      unsafe { ptr::write(buf, Default::default()) }
    }

    #[inline(always)]
    fn activate(
      raw: *mut Self::Raw,
      _args: &mut Self::Args,
      _isolate: &mut Isolate,
      active_scope_data: &mut ActiveScopeData,
    ) -> Self {
      replace(&mut active_scope_data.handle_scope, Self(NonNull::new(raw)))
    }

    #[inline(always)]
    fn get_mut<'a>(
      _isolate: &'a mut Isolate,
      active_scope_data: &'a mut ActiveScopeData,
    ) -> &'a mut Self {
      &mut active_scope_data.handle_scope
    }
  }

  #[derive(Clone, Copy, Default)]
  pub(super) struct EscapeSlot(Option<NonNull<*const super::Value>>);

  impl ScopeData for EscapeSlot {
    type Args = ();
    type Raw = ();

    #[inline(always)]
    fn activate(
      _raw: *mut Self::Raw,
      _args: &mut Self::Args,
      _isolate: &mut Isolate,
      active_scope_data: &mut ActiveScopeData,
    ) -> Self {
      static mut SLOT: *const Value = null();
      let slot_ref = unsafe { &mut SLOT };
      let slot = Self(NonNull::new(slot_ref));
      replace(&mut active_scope_data.escape_slot, slot)
    }

    #[inline(always)]
    fn get_mut<'a>(
      _isolate: &'a mut Isolate,
      active_scope_data: &'a mut ActiveScopeData,
    ) -> &'a mut Self {
      &mut active_scope_data.escape_slot
    }
  }

  impl Deref for EscapeSlot {
    type Target = Option<NonNull<*const super::Value>>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
      &self.0
    }
  }

  impl DerefMut for EscapeSlot {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
      &mut self.0
    }
  }

  #[derive(Clone, Copy, Default)]
  pub(super) struct TryCatch(Option<NonNull<<Self as ScopeData>::Raw>>);

  impl ScopeData for TryCatch {
    type Args = ();
    type Raw = [usize; 5];

    #[inline(always)]
    fn construct(
      buf: *mut Self::Raw,
      _args: &mut Self::Args,
      _isolate: &mut Isolate,
    ) {
      unsafe { ptr::write(buf, Default::default()) }
    }

    #[inline(always)]
    fn activate(
      raw: *mut Self::Raw,
      _args: &mut Self::Args,
      _isolate: &mut Isolate,
      active_scope_data: &mut ActiveScopeData,
    ) -> Self {
      replace(&mut active_scope_data.try_catch, Self(NonNull::new(raw)))
    }

    #[inline(always)]
    fn get_mut<'a>(
      _isolate: &'a mut Isolate,
      active_scope_data: &'a mut ActiveScopeData,
    ) -> &'a mut Self {
      &mut active_scope_data.try_catch
    }
  }

  impl Deref for TryCatch {
    type Target = Option<NonNull<<Self as ScopeData>::Raw>>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
      &self.0
    }
  }

  impl DerefMut for TryCatch {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
      &mut self.0
    }
  }
}

mod internal {
  use super::*;

  pub(super) trait ScopeInit: ScopeParams {
    fn new_with_store(store: Rc<ScopeStore>) -> Self;
  }

  impl<Handles, Escape, TryCatch> ScopeInit for Scope<Handles, Escape, TryCatch> {
    #[inline(always)]
    fn new_with_store(store: Rc<ScopeStore>) -> Self {
      Self {
        store,
        cookie: ScopeCookie::NONE,
        frame_count: 0,
        _phantom: PhantomData,
      }
    }
  }

  pub(crate) struct ScopeStore {
    top_scope_cookie: Cell<ScopeCookie>,
    inner: ScopeStoreInner,
  }

  impl ScopeStore {
    pub fn new(isolate: &mut Isolate) -> Rc<Self> {
      let self_ = Self {
        top_scope_cookie: Default::default(),
        inner: ScopeStoreInner::new(isolate),
      };
      Rc::new(self_)
    }

    #[inline(always)]
    fn with_mut<R>(
      scope: &mut impl ScopeParams,
      f: impl Fn(&mut ScopeStoreInner) -> R,
    ) -> R {
      let scope = scope.as_scope_mut();
      let self_: &Self = &scope.store;
      scope.cookie.borrow(self_.top_scope_cookie.get());
      let result = {
        // This is safe because we can only reach this point when `scope.cookie`
        // matches `top_scope_cookie`. There is only one scope at any time with
        // a matching cookie, and it can only enter here once as its cookie
        // temporarily changes to `ScopeCookie::BORROWED` when it does.
        #[allow(clippy::cast_ref_to_mut)]
        let inner =
          unsafe { &mut *(&self_.inner as *const _ as *mut ScopeStoreInner) };
        // TODO: assigning `scope.frame_count` to `inner.top_scope_frame_count`
        // and back does not seem to get optimized out, even if it should be
        // clear that there is no aliasing taking place. E.g. `to_local()`
        // produces this assembly code:
        //  mov ecx, dword ptr [rdi + 12]  # top_scope_frame_count = frame_count
        //  mov dword ptr [rax + 88], 0
        //  mov dword ptr [rdi + 12], ecx  # frame_count = top_scope_frame_count
        // It should be possible to avoid this.
        debug_assert_eq!(inner.top_scope_frame_count, 0);
        inner.top_scope_frame_count = scope.frame_count;
        let result = f(inner);
        scope.frame_count = take(&mut inner.top_scope_frame_count);
        result
      };
      scope.cookie.unborrow(self_.top_scope_cookie.get());
      result
    }

    #[inline(always)]
    pub fn get<D: ScopeData + Copy, Scope: ScopeParams>(
      scope: &mut Scope,
    ) -> D {
      Self::with_mut(scope, |inner| *inner.get_mut::<D>())
    }

    #[inline(always)]
    pub fn take<D: ScopeData, Scope: ScopeParams>(scope: &mut Scope) -> D {
      Self::with_mut(scope, |inner| take(inner.get_mut::<D>()))
    }

    #[inline(always)]
    fn init_scope_with<Scope: ScopeParams>(
      &self,
      scope: &mut Scope,
      f: impl Fn(&mut ScopeStoreInner) -> (),
    ) {
      //println!("New scope: {}", std::any::type_name::<Scope>());
      let scope = scope.as_scope_mut();

      let next_cookie = ScopeCookie::next(&self.top_scope_cookie);
      ScopeCookie::set(&mut scope.cookie, next_cookie);

      debug_assert_eq!(scope.frame_count, 0);
      Self::with_mut(scope, f);
    }

    #[inline(always)]
    pub(super) fn new_scope_with<'a, Scope: ScopeInit>(
      self: &Rc<Self>,
      f: impl Fn(&mut ScopeStoreInner),
    ) -> Ref<'a, Scope> {
      let mut scope = Scope::new_with_store(self.clone());
      self.init_scope_with(&mut scope, f);
      Ref::<'a, Scope>::new(scope)
    }

    #[inline(always)]
    pub(super) fn new_inner_scope_with<'a, Scope: ScopeInit>(
      parent: &mut impl ScopeParams,
      f: impl Fn(&mut ScopeStoreInner),
    ) -> Ref<'a, Scope> {
      let parent = parent.as_scope_mut();
      assert_eq!(parent.cookie, parent.store.top_scope_cookie.get());
      parent.store.new_scope_with(f)
    }

    #[inline(always)]
    pub fn drop_scope<Scope: ScopeParams>(scope: &mut Scope) {
      //println!("Drop scope: {}", std::any::type_name::<Scope>());
      let scope = scope.as_scope_mut();

      Self::with_mut(scope, |inner| {
        while inner.top_scope_frame_count > 0 {
          inner.pop()
        }
      });
      debug_assert_eq!(scope.frame_count, 0);

      let self_ = &scope.store;
      let cookie = ScopeCookie::revert(&self_.top_scope_cookie);
      ScopeCookie::reset(&mut scope.cookie, cookie);
    }
  }

  impl Drop for ScopeStore {
    fn drop(&mut self) {
      assert_eq!(self.top_scope_cookie.get(), ScopeCookie::default());
    }
  }

  pub(super) struct ScopeStoreInner {
    isolate: *mut Isolate,
    active_scope_data: ActiveScopeData,
    frame_stack: Vec<u8>,
    top_scope_frame_count: u32,
  }

  impl ScopeStoreInner {
    fn new(isolate: &mut Isolate) -> Self {
      Self {
        isolate,
        active_scope_data: Default::default(),
        frame_stack: Vec::with_capacity(Self::FRAME_STACK_SIZE),
        top_scope_frame_count: 0,
      }
    }
  }

  impl Drop for ScopeStoreInner {
    fn drop(&mut self) {
      //println!("Drop ScopeStoreInner")
      assert_eq!(self.top_scope_frame_count, 0);
      assert_eq!(self.frame_stack.len(), 0);
    }
  }

  impl ScopeStoreInner {
    const FRAME_STACK_SIZE: usize = 4096 - size_of::<usize>();

    #[inline(always)]
    pub fn assert_same_isolate(&mut self, isolate: &Isolate) {
      let isolate = isolate as *const _ as *mut Isolate;
      assert_eq!(isolate, self.isolate);
    }

    #[inline(always)]
    pub fn get_mut<D: ScopeData>(&mut self) -> &mut D {
      let isolate = unsafe { &mut *self.isolate };
      D::get_mut(isolate, &mut self.active_scope_data)
    }

    #[inline(always)]
    pub fn push<D: ScopeData>(&mut self, mut args: D::Args) {
      let Self {
        isolate,
        active_scope_data,
        frame_stack,
        top_scope_frame_count,
      } = self;
      let isolate = unsafe { &mut **isolate };

      *top_scope_frame_count += 1;

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

        // Intialize the raw data part of the new stack frame.
        let raw_ptr: *mut D::Raw = &mut (*frame_ptr).raw;
        D::construct(raw_ptr, &mut args, isolate);

        // Update the reference in the ActiveScopeData structure.
        let previous_active =
          D::activate(raw_ptr, &mut args, isolate, active_scope_data);
        let previous_active_ptr: *mut D = &mut (*frame_ptr).previous_active;
        ptr::write(previous_active_ptr, previous_active);

        // Write the metadata part of the new stack frame. It contains the
        // pointer to a cleanup function specific to this type of frame.
        let metadata = ScopeStackFrameMetadata {
          cleanup_fn: Self::cleanup_frame::<D>,
        };
        let metadata_ptr: *mut _ = &mut (*frame_ptr).metadata;
        ptr::write(metadata_ptr, metadata);
      };
    }

    #[inline(always)]
    pub fn pop(&mut self) {
      let Self {
        isolate,
        active_scope_data,
        frame_stack,
        top_scope_frame_count,
      } = self;
      let isolate = unsafe { &mut **isolate };

      debug_assert!(*top_scope_frame_count > 0);
      *top_scope_frame_count -= 1;

      // Locate the metadata part of the stack frame we want to pop.
      let metadata_byte_length = size_of::<ScopeStackFrameMetadata>();
      let metadata_byte_offset = frame_stack.len() - metadata_byte_length;
      let metadata_ptr = frame_stack.get_mut(metadata_byte_offset).unwrap();
      let metadata_ptr: *mut ScopeStackFrameMetadata =
        cast_mut_ptr(metadata_ptr);
      let metadata = unsafe { ptr::read(metadata_ptr) };

      // Call the frame's cleanup handler.
      let cleanup_fn = metadata.cleanup_fn;
      let frame_byte_length =
        unsafe { cleanup_fn(metadata_ptr, isolate, active_scope_data) };
      let frame_byte_offset = frame_stack.len() - frame_byte_length;

      // Decrease the stack limit.
      unsafe { frame_stack.set_len(frame_byte_offset) };
    }

    unsafe fn cleanup_frame<D: ScopeData>(
      metadata_ptr: *mut ScopeStackFrameMetadata,
      isolate: &mut Isolate,
      active_scope_data: &mut ActiveScopeData,
    ) -> usize {
      // From the stack frame metadata pointer, determine the start address of
      // the whole stack frame.
      let frame_byte_length = size_of::<ScopeStackFrame<D>>();
      let metadata_byte_length = size_of::<ScopeStackFrameMetadata>();
      let byte_offset_from_frame = frame_byte_length - metadata_byte_length;
      let frame_address = (metadata_ptr as usize) - byte_offset_from_frame;
      let frame_ptr = frame_address as *mut u8;
      let frame_ptr: *mut ScopeStackFrame<D> = cast_mut_ptr(frame_ptr);

      // Locate the pointers to the other data members within the frame.
      let raw_ptr: *mut D::Raw = &mut (*frame_ptr).raw;
      let previous_active_ptr: *mut D = &mut (*frame_ptr).previous_active;

      // Restore the relevant ActiveScopeData slot to its previous value.
      let previous_active = ptr::read(previous_active_ptr);
      D::deactivate(raw_ptr, previous_active, isolate, active_scope_data);

      // Call the destructor for the raw data part of the frame.
      D::destruct(raw_ptr);

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

  pub(super) trait ScopeData: Default + Sized {
    type Args: Sized;
    type Raw: Sized;

    #[inline(always)]
    fn construct(
      _buf: *mut Self::Raw,
      _args: &mut Self::Args,
      _isolate: &mut Isolate,
    ) {
      assert_eq!(size_of::<Self::Raw>(), 0);
    }

    #[inline(always)]
    fn destruct(raw: *mut Self::Raw) {
      if needs_drop::<Self::Raw>() {
        unsafe { drop_in_place(raw) }
      }
    }

    fn activate(
      raw: *mut Self::Raw,
      args: &mut Self::Args,
      _isolate: &mut Isolate,
      active_scope_data: &mut ActiveScopeData,
    ) -> Self;

    #[inline(always)]
    fn deactivate(
      _raw: *mut Self::Raw,
      previous: Self,
      isolate: &mut Isolate,
      active_scope_data: &mut ActiveScopeData,
    ) {
      replace(Self::get_mut(isolate, active_scope_data), previous);
    }

    fn get_mut<'a>(
      _isolate: &'a mut Isolate,
      active_scope_data: &'a mut ActiveScopeData,
    ) -> &'a mut Self;
  }

  #[derive(Default)]
  pub(super) struct ActiveScopeData {
    pub context: data::Context,
    pub handle_scope: data::HandleScope,
    pub escape_slot: data::EscapeSlot,
    pub try_catch: data::TryCatch,
  }

  struct ScopeStackFrame<D: ScopeData> {
    raw: D::Raw,
    previous_active: D,
    metadata: ScopeStackFrameMetadata,
  }

  struct ScopeStackFrameMetadata {
    cleanup_fn:
      unsafe fn(*mut Self, &mut Isolate, &mut ActiveScopeData) -> usize,
  }

  #[repr(transparent)]
  #[derive(Copy, Clone, Debug, Eq, PartialEq)]
  pub(super) struct ScopeCookie(u32);

  impl ScopeCookie {
    pub const NONE: Self = Self(0);
    pub const BORROWED: Self = Self(!0);

    #[inline(always)]
    fn next(cell: &Cell<Self>) -> Self {
      let cur_cookie = cell.get();
      let next_cookie = Self(cur_cookie.0 + 1);
      cell.set(next_cookie);
      next_cookie
    }

    #[inline(always)]
    fn revert(cell: &Cell<Self>) -> Self {
      let cur_cookie = cell.get();
      assert_ne!(cur_cookie, Self::default());
      let old_cookie = Self(cur_cookie.0 - 1);
      cell.set(old_cookie);
      cur_cookie
    }

    #[inline(always)]
    fn set(&mut self, value: Self) {
      let invalid = replace(self, value);
      assert_eq!(invalid, Self::NONE)
    }

    #[inline(always)]
    fn reset(&mut self, expected_value: Self) {
      let cookie = replace(self, Self::NONE);
      assert_eq!(cookie, expected_value);
    }

    #[inline(always)]
    fn borrow(&mut self, expected_value: Self) {
      let cookie = replace(self, Self::BORROWED);
      assert_eq!(cookie, expected_value);
    }

    #[inline(always)]
    fn unborrow(&mut self, value: Self) {
      let cookie = replace(self, value);
      assert_eq!(cookie, Self::BORROWED);
    }
  }
}

impl Default for ScopeCookie {
  fn default() -> Self {
    Self::NONE
  }
}

#[derive(Copy, Clone)]
struct Value(*mut ());

#[derive(Copy, Clone)]
pub struct Context(*mut ());

impl Context {
  pub fn get_isolate(&self) -> &Isolate {
    unimplemented!()
  }
}

#[derive(Clone)]
pub struct Isolate {
  scopes: Rc<ScopeStore>,
}

impl Isolate {
  pub fn new() -> Box<Self> {
    new_box_with(|isolate| {
      let scopes = ScopeStore::new(unsafe { &mut *isolate });
      Self { scopes }
    })
  }

  fn get_scopes(&self) -> &Rc<ScopeStore> {
    &self.scopes
  }
}

fn new_box_with<T>(new_fn: impl FnOnce(*mut T) -> T) -> Box<T> {
  let b = Box::new(std::mem::MaybeUninit::<T>::uninit());
  let p = Box::into_raw(b) as *mut T;
  unsafe { ptr::write(p, new_fn(p)) };
  unsafe { Box::from_raw(p) }
}

impl Isolate {
  fn get_current_context(&self) -> Option<Local<Context>> {
    unimplemented!()
  }
}

#[derive(Copy, Clone)]
pub struct Local<'a, T> {
  ptr: *const T,
  _phantom: PhantomData<&'a T>,
}

impl<'a, T> Local<'a, T> {
  fn from_raw(ptr: *const T) -> Self {
    Self {
      ptr,
      _phantom: PhantomData,
    }
  }
}

impl<'a, T> Default for Local<'a, T> {
  fn default() -> Self {
    Local {
      _phantom: PhantomData,
      ptr: null(),
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
    let addr = 42usize * size_of::<T>();
    scope.to_local::<T>(addr as *const _)
  }
}

impl<'a, T> Deref for Local<'a, T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    unsafe { &*self.ptr }
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
  let isolate = Isolate::new();
  let root = &mut Scope::from_isolate(&isolate);
  let hs = &mut Scope::handle_scope(root);
  let esc1 = &mut Scope::escapable_handle_scope(hs);
  let esc2 = &mut EscapableHandleScope::new(esc1);
  let ehs = &mut Scope::handle_scope(esc2);
  let l1 = Local::<Value>::new(ehs);
  let e1 = ehs.escape(l1);
  let tc = &mut TryCatch::new(ehs);
  create_local_in_escapable_handle_scope(tc);
  let tcl1 = Local::<Value>::new(tc);
  {
    let tce = &mut EscapableHandleScope::new(tc);
    let e1 = tce.escape(l1);
  }
  let hs = &mut Scope::handle_scope(tc);
}

fn main() {
  testing();

  let isolate1 = Isolate::new();
  let root1 = &mut Scope::from_isolate(&isolate1);
  let isolate2 = Isolate::new();
  let root2 = &mut Scope::from_isolate(&isolate2);
  {
    let x = &mut Scope::handle_scope(root1);
    let _xxv = Local::<Value>::new(x);
    let yyv = {
      let mut y = HandleScope::new(x);
      //std::mem::swap(&mut x, &mut y);
      //let r1 = Local::<Value>::new(x);
      //let r2 = (y.get_make_local())();
      let r1 = Local::<Value>::new(&mut y);
      let r2 = Local::<Value>::new(&mut y);
      let r3 = Local::<Value>::new(&mut y);
      {
        let sc = &mut Scope::from_isolate(&isolate1);
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
  let isolate = Isolate::new();
  let root = &mut Scope::from_isolate(&isolate);
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
