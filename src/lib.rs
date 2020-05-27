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
  type NewScope = alloc::ContextScope<'a, active::EscapableHandleScope<'c, 'd>>;
}

impl<'a, 'b: 'a, 'c: 'b> AddContextScope<'a>
  for active::TryCatch<'b, active::HandleScope<'c>>
{
  type NewScope = alloc::ContextScope<'a, active::HandleScope<'c>>;
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
    innermost: Option<NonNull<Innermost>>,
    isolate: Option<NonNull<Isolate>>,
    context: Option<NonNull<Context>>,
    escape_slot: Option<NonNull<Address>>,
    try_catch: Option<NonNull<data2::TryCatch>>,
  }

  struct ScopeData<'a, C1: ScopeComponent = (), C2: ScopeComponent = ()> {
    effective: NonNull<EffectiveScope>,
    effective_prior: Option<NonNull<EffectiveScope>>,
    component1: C1,
    component2: C2,
    _phantom: PhantomData<&'a mut ()>,
  }

  impl<'a, C1: ScopeComponent, C2: ScopeComponent> ScopeData<'a, C1, C2> {
    fn enter(&mut self) {
      let effective_scope = unsafe { self.effective.as_mut() };
      self.component2.enter(effective_scope);
      self.component1.enter(effective_scope);
    }
    fn exit(&mut self) {
      let effective_scope = unsafe { self.effective.as_mut() };
      self.component1.exit(effective_scope);
      self.component2.exit(effective_scope);
    }
  }
  impl<'a, C1: ScopeComponent, C2: ScopeComponent> Drop
    for ScopeData<'a, C1, C2>
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
    fn new(_effective_scope: &mut EffectiveScope) -> Self
    where
      Self: Default,
    {
      Default::default()
    }
    fn enter(&mut self, _effective_scope: &mut EffectiveScope) {}
    fn exit(&mut self, _effective_scope: &mut EffectiveScope) {}
    fn as_non_null(&mut self) -> NonNull<Self> {
      unsafe { NonNull::new_unchecked(self) }
    }
  }

  impl ScopeComponent for () {}

  #[derive(Eq, PartialEq)]
  struct Innermost {
    prior: Option<NonNull<Innermost>>,
    data: NonNull<EffectiveScope>,
  }
  impl Innermost {
    fn scope_has_been_entered(&mut self) -> bool {
      let self_ptr = self.as_non_null();
      let effective_scope = unsafe { self.data.as_ref() };
      match effective_scope.innermost {
        Some(p) if p == self_ptr => true,
        p if p == self.prior => false,
        _ => panic!("cannot use scope while it is shadowed"),
      }
    }
  }
  impl ScopeComponent for Innermost {
    fn enter(&mut self, effective_scope: &mut EffectiveScope) {
      let entered = effective_scope.innermost.replace(self.as_non_null());
      match &mut self.prior {
        prior @ Some(_) => assert_eq!(*prior, entered),
        prior @ None => *prior = entered,
      }
    }
    fn exit(&mut self, effective_scope: &mut EffectiveScope) {
      let left = replace(&mut effective_scope.innermost, self.prior).unwrap();
      assert_eq!(left, self.as_non_null());
    }
  }

  #[repr(C)]
  struct ContextScope {
    prior: Option<NonNull<Context>>,
    context: NonNull<Context>,
  }
  impl ScopeComponent for ContextScope {
    fn enter(&mut self, effective_scope: &mut EffectiveScope) {
      // XXX enter Context.
      let c = effective_scope.context.replace(self.context);
      let c = replace(&mut self.prior, c);
      assert!(c.is_none());
    }
    fn exit(&mut self, effective_scope: &mut EffectiveScope) {
      let c = self.prior.take();
      let c = replace(&mut effective_scope.context, c).unwrap();
      assert_eq!(c, self.context);
      // XXX exit Context.
    }
  }

  #[repr(C)]
  struct HandleScope {
    raw: raw::HandleScope,
  }
  impl ScopeComponent for HandleScope {
    fn enter(&mut self, _effective_scope: &mut EffectiveScope) {
      // Create raw handlescope.
    }
    fn exit(&mut self, _effective_scope: &mut EffectiveScope) {
      // Destroy raw handlescope.
    }
  }

  struct EscapeSlot {
    prior: Option<NonNull<Address>>,
    slot: NonNull<Address>,
  }
  impl ScopeComponent for EscapeSlot {
    fn enter(&mut self, effective_scope: &mut EffectiveScope) {
      // XXX Create raw slot.
      let e = effective_scope.escape_slot.replace(self.slot);
      let e = replace(&mut self.prior, e);
      assert!(e.is_none());
    }
    fn exit(&mut self, effective_scope: &mut EffectiveScope) {
      let e = self.prior.take();
      let e = replace(&mut effective_scope.escape_slot, e).unwrap();
      assert_eq!(e, self.slot);
      // XXX Destroy raw slot.
    }
  }

  #[repr(C)]
  struct TryCatch {
    prior: Option<NonNull<TryCatch>>,
    raw: NonNull<TryCatch>,
  }
  impl ScopeComponent for TryCatch {
    fn enter(&mut self, effective_scope: &mut EffectiveScope) {
      // XXX Create raw trycatch.
      let tc = effective_scope.try_catch.replace(self.as_non_null());
      let tc = replace(&mut self.prior, tc);
      assert!(tc.is_none());
    }
    fn exit(&mut self, effective_scope: &mut EffectiveScope) {
      let tc = self.prior.take();
      let tc = replace(&mut effective_scope.try_catch, tc).unwrap();
      assert_eq!(tc, self.as_non_null());
      // XXX Destroy raw trycatch.
    }
  }
}

mod raw {
  pub type Address = usize;
  #[repr(C)]
  pub struct HandleScope([usize; 3]);
  #[repr(C)]
  pub struct TryCatch([usize; 6]);
}

#[doc(inline)]
pub use active::*;
