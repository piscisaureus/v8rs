#![allow(dead_code)]

use std::marker::PhantomData;
use std::mem::replace;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::null_mut;
use std::ptr::NonNull;

pub struct Context {
  ptr: *mut (),
}
impl Context {
  pub fn new() -> Self {
    Self { ptr: null_mut() }
  }
}

//struct IsolateAnnex {
//  context: Option<NonNull<Context>>,
//  escape_slot: Option<NonNull<usize>>,
//  try_catch: Option<NonNull<TryCatch>>,
//}

pub struct Local<'a, T> {
  ptr: *mut (),
  _phantom: PhantomData<&'a T>,
}
impl<'a, T> Local<'a, T> {
  pub fn new(_: &'_ mut HandleScope<'a>) -> Self {
    Self {
      ptr: null_mut(),
      _phantom: PhantomData,
    }
  }
}

pub trait AddContextScope<'a> {
  type NewScope;
}
pub trait AddHandleScope<'a> {
  type NewScope;
}
pub trait AddEscapableHandleScope<'a> {
  type NewScope;
}
pub trait AddTryCatch<'a> {
  type NewScope;
}

// ===== ContextScope<'a> =====

impl<'a, 'b: 'a> AddContextScope<'a> for active::HandleScope<'b, ()> {
  type NewScope = alloc::ContextScope<'a, active::HandleScope<'b>>;
}

impl<'a, 'b: 'a> AddContextScope<'a> for active::HandleScope<'b> {
  type NewScope = alloc::ContextScope<'a, active::HandleScope<'b>>;
}

impl<'a, 'b: 'a, 'c: 'b> AddContextScope<'a>
  for active::EscapableHandleScope<'b, 'c>
{
  type NewScope = alloc::ContextScope<'a, active::EscapableHandleScope<'b, 'c>>;
}

impl<'a, 'b: 'a, 'c: 'b, 'd: 'c> AddContextScope<'a>
  for active::TryCatch<'b, active::EscapableHandleScope<'c, 'd>>
{
  type NewScope = alloc::ContextScope<
    'a,
    active::TryCatch<'b, active::EscapableHandleScope<'c, 'd>>,
  >;
}

impl<'a, 'b: 'a, 'c: 'b> AddContextScope<'a>
  for active::TryCatch<'b, active::HandleScope<'c>>
{
  type NewScope =
    alloc::ContextScope<'a, active::TryCatch<'b, active::HandleScope<'c>>>;
}

// ===== HandleScope<'a> =====

impl<'a, 'b: 'a> AddHandleScope<'a> for active::ContextScope<'b> {
  type NewScope = alloc::HandleScope<'a>;
}

impl<'a, 'b: 'a, P: AddHandleScope<'a>> AddHandleScope<'a>
  for active::ContextScope<'b, P>
{
  type NewScope = <P as AddHandleScope<'a>>::NewScope;
}

impl<'a, 'b: 'a> AddHandleScope<'a> for active::HandleScope<'b> {
  type NewScope = alloc::HandleScope<'a>;
}

impl<'a, 'b: 'a, 'c: 'b> AddHandleScope<'a>
  for active::EscapableHandleScope<'b, 'c>
{
  type NewScope = alloc::EscapableHandleScope<'a, 'c>;
}

impl<'a, 'b: 'a, 'c: 'b, 'd: 'c> AddHandleScope<'a>
  for active::TryCatch<'b, active::EscapableHandleScope<'c, 'd>>
{
  type NewScope = alloc::EscapableHandleScope<'a, 'd>;
}

impl<'a, 'b: 'a, 'c: 'b> AddHandleScope<'a>
  for active::TryCatch<'b, active::HandleScope<'c>>
{
  type NewScope = alloc::HandleScope<'a>;
}

// ===== EscapableHandleScope<'a, 'b> =====

impl<'a, 'b: 'a, P: AddEscapableHandleScope<'a>> AddEscapableHandleScope<'a>
  for active::ContextScope<'b, P>
{
  type NewScope = <P as AddEscapableHandleScope<'a>>::NewScope;
}

impl<'a, 'b: 'a> AddEscapableHandleScope<'a> for active::HandleScope<'b> {
  type NewScope = alloc::EscapableHandleScope<'a, 'b>;
}

impl<'a, 'b: 'a, 'c: 'b> AddEscapableHandleScope<'a>
  for active::EscapableHandleScope<'b, 'c>
{
  type NewScope = alloc::EscapableHandleScope<'a, 'b>;
}

impl<'a, 'b: 'a, 'c: 'b, 'd: 'c> AddEscapableHandleScope<'a>
  for active::TryCatch<'b, active::EscapableHandleScope<'c, 'd>>
{
  type NewScope = alloc::EscapableHandleScope<'a, 'c>;
}

impl<'a, 'b: 'a, 'c: 'b> AddEscapableHandleScope<'a>
  for active::TryCatch<'b, active::HandleScope<'c>>
{
  type NewScope = alloc::EscapableHandleScope<'a, 'c>;
}

// ===== TryCatch<'a> =====

impl<'a, 'b: 'a, P: AddTryCatch<'a>> AddTryCatch<'a>
  for active::ContextScope<'b, P>
{
  type NewScope = <P as AddTryCatch<'a>>::NewScope;
}

impl<'a, 'b: 'a> AddTryCatch<'a> for active::HandleScope<'b> {
  type NewScope = alloc::TryCatch<'a, active::HandleScope<'b>>;
}

impl<'a, 'b: 'a, 'c: 'b> AddTryCatch<'a>
  for active::EscapableHandleScope<'b, 'c>
{
  type NewScope = alloc::TryCatch<'a, active::EscapableHandleScope<'b, 'c>>;
}

pub(self) mod data {
  use super::*;
  pub struct ContextScope(NonNull<Context>);
  pub struct EscapeSlot(*const ());
  pub struct HandleScope([usize; 3]);
  pub struct EscapableHandleScope {
    handle_scope: HandleScope,
    escape_slot: EscapeSlot,
  }
  pub(crate) struct TryCatch([usize; 7]);

  impl Drop for HandleScope {
    fn drop(&mut self) {}
  }
  impl Drop for ContextScope {
    fn drop(&mut self) {}
  }
  impl Drop for EscapableHandleScope {
    fn drop(&mut self) {}
  }
  impl Drop for TryCatch {
    fn drop(&mut self) {}
  }
}

pub mod alloc {
  use super::*;
  pub enum ContextScope<'a, P> {
    Declared {
      parent: &'a mut P,
      context: &'a Context,
    },
    Entered(data::ContextScope),
  }
  pub enum HandleScope<'a, P = Context> {
    Declared(&'a mut P),
    Entered(data::HandleScope),
  }
  pub enum EscapableHandleScope<'a, 'b, P = Context> {
    Declared {
      parent: &'a mut P,
      escape_slot: &'b mut (),
    },
    Entered(data::EscapableHandleScope),
  }
  pub enum TryCatch<'a, P = Context> {
    Declared(&'a mut P),
    Entered(data::HandleScope),
  }

  impl<'a, P> ContextScope<'a, P> {
    pub fn enter(&'a mut self) -> &'a mut active::ContextScope<'a, P> {
      unimplemented!()
    }
  }
  impl<'a> HandleScope<'a, ()> {
    pub fn enter(&'a mut self) -> &'a mut active::HandleScope<'a, ()> {
      unimplemented!()
    }
  }
  impl<'a> HandleScope<'a, Context> {
    pub fn enter(&'a mut self) -> &'a mut active::HandleScope<'a, Context> {
      unimplemented!()
    }
  }
  impl<'a, 'b> EscapableHandleScope<'a, 'b> {
    pub fn enter(&'a mut self) -> &'a mut active::EscapableHandleScope<'a, 'b> {
      unimplemented!()
    }
  }
  impl<'a, 'b, 'c> TryCatch<'a, active::EscapableHandleScope<'b, 'c>> {
    pub fn enter(
      &'a mut self,
    ) -> &'a mut active::TryCatch<'a, active::EscapableHandleScope<'b, 'c>>
    {
      unimplemented!()
    }
  }
  impl<'a, 'b> TryCatch<'a, active::HandleScope<'b, Context>> {
    pub fn enter(
      &'a mut self,
    ) -> &'a mut active::TryCatch<'a, active::HandleScope<'b, Context>> {
      unimplemented!()
    }
  }
}

pub(self) mod active {
  use super::*;

  pub struct ContextScope<'a, P = ()> {
    pub(super) effective_scope: NonNull<EffectiveScope>,
    _phantom: PhantomData<&'a mut P>,
  }
  pub struct HandleScope<'a, P = Context> {
    pub(super) effective_scope: NonNull<EffectiveScope>,
    _phantom: PhantomData<&'a mut P>,
  }
  pub struct EscapableHandleScope<'a, 'b> {
    pub(super) effective_scope: NonNull<EffectiveScope>,
    _phantom: PhantomData<(&'a mut (), &'b mut ())>,
  }
  pub struct TryCatch<'a, P = Context> {
    pub(super) effective_scope: NonNull<EffectiveScope>,
    _phantom: PhantomData<&'a mut P>,
  }

  impl<'a> ContextScope<'a> {
    pub fn root(_context: &'a Context) -> alloc::ContextScope<'a, ()> {
      unimplemented!()
    }
    pub fn new<'b: 'a, P: AddContextScope<'a> + 'b>(
      _parent: &'a mut P,
      _context: &'a Context,
    ) -> <P as AddContextScope<'a>>::NewScope {
      unimplemented!()
    }
  }
  impl<'a> HandleScope<'a> {
    pub fn root() -> alloc::HandleScope<'a, ()> {
      unimplemented!()
    }
    pub fn new<'b: 'a, P: AddHandleScope<'a> + 'b>(
      _parent: &'a mut P,
    ) -> <P as AddHandleScope<'a>>::NewScope {
      unimplemented!()
    }
  }
  impl<'a, 'b> EscapableHandleScope<'a, 'b> {
    pub fn new<'c: 'a, P: AddEscapableHandleScope<'a> + 'c>(
      _parent: &'a mut P,
    ) -> <P as AddEscapableHandleScope<'a>>::NewScope {
      unimplemented!()
    }
  }
  impl<'a> TryCatch<'a> {
    pub fn new<'b: 'a, P: AddTryCatch<'a> + 'b>(
      _parent: &'a mut P,
    ) -> <P as AddTryCatch<'a>>::NewScope {
      unimplemented!()
    }
  }

  impl<'a, P> Drop for ContextScope<'a, P> {
    fn drop(&mut self) {}
  }
  impl<'a, P> Drop for HandleScope<'a, P> {
    fn drop(&mut self) {}
  }
  impl<'a, 'b> Drop for EscapableHandleScope<'a, 'b> {
    fn drop(&mut self) {}
  }
  impl<'a, P> Drop for TryCatch<'a, P> {
    fn drop(&mut self) {}
  }

  impl<'a, P> Deref for ContextScope<'a, P> {
    type Target = P;
    fn deref(&self) -> &Self::Target {
      unsafe { &*(self as *const _ as *const Self::Target) }
    }
  }

  impl<'a, P> DerefMut for ContextScope<'a, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
      unsafe { &mut *(self as *mut _ as *mut Self::Target) }
    }
  }

  impl<'a> Deref for HandleScope<'a> {
    type Target = ContextScope<'a, ()>;
    fn deref(&self) -> &Self::Target {
      unsafe { &*(self as *const _ as *const Self::Target) }
    }
  }

  impl<'a> DerefMut for HandleScope<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
      unsafe { &mut *(self as *mut _ as *mut Self::Target) }
    }
  }

  impl<'a, 'b> Deref for EscapableHandleScope<'a, 'b> {
    type Target = HandleScope<'a>;
    fn deref(&self) -> &Self::Target {
      unsafe { &*(self as *const _ as *const Self::Target) }
    }
  }

  impl<'a, 'b> DerefMut for EscapableHandleScope<'a, 'b> {
    fn deref_mut(&mut self) -> &mut Self::Target {
      unsafe { &mut *(self as *mut _ as *mut Self::Target) }
    }
  }

  impl<'a, 'b, 'c> Deref for TryCatch<'a, EscapableHandleScope<'b, 'c>> {
    type Target = EscapableHandleScope<'b, 'c>;
    fn deref(&self) -> &Self::Target {
      unsafe { &*(self as *const _ as *const Self::Target) }
    }
  }

  impl<'a, 'b, 'c> DerefMut for TryCatch<'a, EscapableHandleScope<'b, 'c>> {
    fn deref_mut(&mut self) -> &mut Self::Target {
      unsafe { &mut *(self as *mut _ as *mut Self::Target) }
    }
  }

  impl<'a, 'b> Deref for TryCatch<'a, HandleScope<'b>> {
    type Target = HandleScope<'b>;
    fn deref(&self) -> &Self::Target {
      unsafe { &*(self as *const _ as *const Self::Target) }
    }
  }

  impl<'a, 'b> DerefMut for TryCatch<'a, HandleScope<'b>> {
    fn deref_mut(&mut self) -> &mut Self::Target {
      unsafe { &mut *(self as *mut _ as *mut Self::Target) }
    }
  }

  impl<'a> HandleScope<'a, ()> {}
  impl<'a> HandleScope<'a, Context> {}
  impl<'a, 'b> EscapableHandleScope<'a, 'b> {}
  impl<'a, 'b, 'c> TryCatch<'a, EscapableHandleScope<'b, 'c>> {}
  impl<'a, 'b> TryCatch<'a, HandleScope<'b, Context>> {}
}

use data2::EffectiveScope;
mod data2 {
  use super::*;

  struct Isolate();
  type Address = usize;

  pub struct EffectiveScope {
    topmost_scope: Option<NonNull<NonNull<EffectiveScope>>>,
    isolate: Option<NonNull<Isolate>>,
    context: Option<NonNull<Context>>,
    escape_slot: Option<NonNull<Address>>,
    try_catch: Option<NonNull<TryCatch>>,
  }

  struct ScopeData<
    'a,
    P: ScopeParent<'a>,
    C: ScopeComponent = (),
    H: ScopeComponent = (),
    E: ScopeComponent = (),
    T: ScopeComponent = (),
  > {
    scope_prior: P::Prior,
    effective_scope: NonNull<EffectiveScope>,
    context_scope: C,
    context_scope_prior: C::Prior,
    handle_scope: H,
    handle_scope_prior: H::Prior,
    escape_slot: E,
    escape_slot_prior: E::Prior,
    try_catch: T,
    try_catch_prior: T::Prior,
  }

  impl<
      'a,
      P: ScopeParent<'a>,
      C: ScopeComponent,
      H: ScopeComponent,
      E: ScopeComponent,
      T: ScopeComponent,
    > ScopeData<'a, P, C, H, E, T>
  {
    fn enter(&mut self) {
      let effective_scope = unsafe { self.effective_scope.as_mut() };
      self
        .context_scope
        .enter(&mut self.context_scope_prior, effective_scope);
      self
        .escape_slot
        .enter(&mut self.escape_slot_prior, effective_scope);
      self
        .handle_scope
        .enter(&mut self.handle_scope_prior, effective_scope);
      self
        .try_catch
        .enter(&mut self.try_catch_prior, effective_scope);
    }
    fn exit(&mut self) {
      let effective_scope = unsafe { self.effective_scope.as_mut() };
      self
        .context_scope
        .exit(&mut self.context_scope_prior, effective_scope);
      self
        .escape_slot
        .exit(&mut self.escape_slot_prior, effective_scope);
      self
        .handle_scope
        .exit(&mut self.handle_scope_prior, effective_scope);
      self
        .try_catch
        .exit(&mut self.try_catch_prior, effective_scope);
    }
  }
  impl<
      'a,
      P: ScopeParent<'a>,
      C: ScopeComponent,
      H: ScopeComponent,
      E: ScopeComponent,
      T: ScopeComponent,
    > Drop for ScopeData<'a, P, C, H, E, T>
  {
    fn drop(&mut self) {}
  }

  trait ScopeParent<'a> {
    type Prior: 'a;
    fn prior(&'a mut self) -> Self::Prior;
  }
  impl<'a> ScopeParent<'a> for () {
    type Prior = ();
    fn prior(&'a mut self) -> Self::Prior {}
  }
  impl<'a, 'b: 'a> ScopeParent<'a> for &'a mut active::HandleScope<'b, ()> {
    type Prior = &'a mut NonNull<EffectiveScope>;
    fn prior(&'a mut self) -> Self::Prior {
      &mut self.effective_scope
    }
  }
  impl<'a, 'b: 'a> ScopeParent<'a> for &'a mut active::HandleScope<'b> {
    type Prior = &'a mut NonNull<EffectiveScope>;
    fn prior(&'a mut self) -> Self::Prior {
      &mut self.effective_scope
    }
  }
  impl<'a, 'b: 'a, 'c: 'b> ScopeParent<'a>
    for &'a mut active::EscapableHandleScope<'b, 'c>
  {
    type Prior = &'a mut NonNull<EffectiveScope>;
    fn prior(&'a mut self) -> Self::Prior {
      &mut self.effective_scope
    }
  }
  impl<'a, 'b: 'a, H> ScopeParent<'a> for &'a mut active::TryCatch<'b, H> {
    type Prior = &'a mut NonNull<EffectiveScope>;
    fn prior(&'a mut self) -> Self::Prior {
      &mut self.effective_scope
    }
  }

  trait ScopeComponent {
    type Prior: Default;
    fn enter(
      &mut self,
      _prior: &mut Self::Prior,
      _effective_scope: &mut EffectiveScope,
    ) {
    }
    fn exit(
      &mut self,
      _prior: &mut Self::Prior,
      _effective_scope: &mut EffectiveScope,
    ) {
    }
  }

  impl ScopeComponent for () {
    type Prior = ();
    fn enter(
      &mut self,
      _prior: &mut Self::Prior,
      _effective_scope: &mut EffectiveScope,
    ) {
    }
    fn exit(
      &mut self,
      _prior: &mut Self::Prior,
      _effective_scope: &mut EffectiveScope,
    ) {
    }
  }

  #[repr(C)]
  struct ContextScope(NonNull<Context>);
  impl ScopeComponent for ContextScope {
    type Prior = Option<NonNull<Context>>;
    fn enter(
      &mut self,
      prior: &mut Self::Prior,
      effective_scope: &mut EffectiveScope,
    ) {
      // XXX enter Context.
      let ctx = effective_scope.context.replace(self.0);
      let ctx = replace(prior, ctx);
      assert!(ctx.is_none());
    }
    fn exit(
      &mut self,
      prior: &mut Self::Prior,
      effective_scope: &mut EffectiveScope,
    ) {
      let ctx = prior.take();
      let ctx = replace(&mut effective_scope.context, ctx).unwrap();
      assert_eq!(ctx, self.0);
      // XXX exit Context.
    }
  }

  #[repr(C)]
  struct HandleScope([usize; 3]);
  impl ScopeComponent for HandleScope {
    type Prior = ();
    fn enter(
      &mut self,
      _prior: &mut Self::Prior,
      _effective_scope: &mut EffectiveScope,
    ) {
      // Create raw handlescope.
    }
    fn exit(
      &mut self,
      _prior: &mut Self::Prior,
      _effective_scope: &mut EffectiveScope,
    ) {
      // Destroy raw handlescope.
    }
  }

  #[repr(transparent)]
  struct EscapeSlot(NonNull<Address>);
  impl ScopeComponent for EscapeSlot {
    type Prior = Option<NonNull<Address>>;
    fn enter(
      &mut self,
      prior: &mut Self::Prior,
      effective_scope: &mut EffectiveScope,
    ) {
      // XXX Create raw slot.
      let esc = effective_scope.escape_slot.replace(self.0);
      let esc = replace(prior, esc);
      assert!(esc.is_none());
    }
    fn exit(
      &mut self,
      prior: &mut Self::Prior,
      effective_scope: &mut EffectiveScope,
    ) {
      let esc = prior.take();
      let esc = replace(&mut effective_scope.escape_slot, esc).unwrap();
      assert_eq!(esc, self.0);
      // XXX Destroy raw slot.
    }
  }

  #[repr(C)]
  struct TryCatch([usize; 6]);
  impl ScopeComponent for TryCatch {
    type Prior = Option<NonNull<TryCatch>>;
    fn enter(
      &mut self,
      prior: &mut Self::Prior,
      effective_scope: &mut EffectiveScope,
    ) {
      // XXX Create raw trycatch.
      let tc = effective_scope.try_catch.replace(NonNull::from(self));
      let tc = replace(prior, tc);
      assert!(tc.is_none());
    }
    fn exit(
      &mut self,
      prior: &mut Self::Prior,
      effective_scope: &mut EffectiveScope,
    ) {
      let tc = prior.take();
      let tc = replace(&mut effective_scope.try_catch, tc).unwrap();
      assert_eq!(tc, NonNull::from(self));
      // XXX Destroy raw trycatch.
    }
  }
}

mod raw {
  #[repr(C)]
  struct HandleScope([usize; 3]);
  #[repr(C)]
  struct TryCatch([usize; 6]);
}

#[doc(inline)]
pub use active::*;
