#![allow(dead_code)]

mod mock;

pub use mock::*;
use std::marker::PhantomData;
use std::mem::align_of;
use std::mem::replace;
use std::mem::size_of;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::NonNull;

pub trait GetEffectiveScopeForRoot {
  fn get_isolate(&self) -> NonNull<Isolate>;
  fn get_effective_scope(&self) -> NonNull<EffectiveScope> {
    let isolate = self.get_isolate();
    let isolate = unsafe { isolate.as_ref() };
    let annex = isolate.get_annex();
    unsafe { NonNull::new_unchecked(annex as *const _ as *mut EffectiveScope) }
  }
}
impl GetEffectiveScopeForRoot for Isolate {
  fn get_isolate(&self) -> NonNull<Isolate> {
    unsafe { NonNull::new_unchecked(self as *const _ as *mut Isolate) }
  }
}
impl GetEffectiveScopeForRoot for Context {
  fn get_isolate(&self) -> NonNull<Isolate> {
    let p = unsafe { raw::v8__Context__GetIsolate(self) };
    NonNull::new(p).unwrap()
  }
}

pub trait RootHandleScopeParam<'a>: GetEffectiveScopeForRoot {
  type Data;
  fn root(&'a self) -> Self::Data;
}
impl<'a> RootHandleScopeParam<'a> for Isolate {
  type Data = data2::HandleScope<'a, active::HandleScope<'a, Isolate>>;
  fn root(&'a self) -> Self::Data {
    data2::HandleScope::root(self)
  }
}
impl<'a> RootHandleScopeParam<'a> for Context {
  type Data = data2::ContextAndHandleScope<'a, active::HandleScope<'a>>;
  fn root(&'a self) -> Self::Data {
    data2::ContextAndHandleScope::root(self)
  }
}

pub trait GetEffectiveScope {
  fn get_effective_scope(&mut self) -> NonNull<EffectiveScope>;
}
impl<'a, P> GetEffectiveScope for active::ContextScope<'a, P> {
  fn get_effective_scope(&mut self) -> NonNull<EffectiveScope> {
    self.effective_scope
  }
}
impl<'a> GetEffectiveScope for active::HandleScope<'a, Isolate> {
  fn get_effective_scope(&mut self) -> NonNull<EffectiveScope> {
    self.effective_scope
  }
}
impl<'a> GetEffectiveScope for active::HandleScope<'a, Context> {
  fn get_effective_scope(&mut self) -> NonNull<EffectiveScope> {
    self.effective_scope
  }
}
impl<'a, 'b> GetEffectiveScope for active::EscapableHandleScope<'a, 'b> {
  fn get_effective_scope(&mut self) -> NonNull<EffectiveScope> {
    self.effective_scope
  }
}
impl<'a, 'b> GetEffectiveScope
  for active::TryCatch<'a, active::HandleScope<'b, Context>>
{
  fn get_effective_scope(&mut self) -> NonNull<EffectiveScope> {
    self.effective_scope
  }
}
impl<'a, 'b, 'c> GetEffectiveScope
  for active::TryCatch<'a, active::EscapableHandleScope<'b, 'c>>
{
  fn get_effective_scope(&mut self) -> NonNull<EffectiveScope> {
    self.effective_scope
  }
}

pub trait AddContextScope<'a>: GetEffectiveScope {
  type NewScope: GetEffectiveScope;
}
pub trait AddHandleScope<'a>: GetEffectiveScope {
  type NewScope: GetEffectiveScope;
}
pub trait AddEscapableHandleScope<'a>: GetEffectiveScope {
  type NewScope: GetEffectiveScope;
}
pub trait AddTryCatch<'a>: GetEffectiveScope {
  type NewScope: GetEffectiveScope;
}

// ===== ContextScope<'a> =====

impl<'a, 'b: 'a> AddContextScope<'a> for active::HandleScope<'b, Isolate> {
  type NewScope = active::ContextScope<'a, active::HandleScope<'b>>;
}

impl<'a, 'b: 'a> AddContextScope<'a> for active::HandleScope<'b> {
  type NewScope = active::ContextScope<'a, active::HandleScope<'b>>;
}

impl<'a, 'b: 'a, 'c: 'b> AddContextScope<'a>
  for active::EscapableHandleScope<'b, 'c>
{
  type NewScope =
    active::ContextScope<'a, active::EscapableHandleScope<'b, 'c>>;
}

impl<'a, 'b: 'a, 'c: 'b, 'd: 'c> AddContextScope<'a>
  for active::TryCatch<'b, active::EscapableHandleScope<'c, 'd>>
{
  type NewScope =
    active::ContextScope<'a, active::EscapableHandleScope<'c, 'd>>;
}

impl<'a, 'b: 'a, 'c: 'b> AddContextScope<'a>
  for active::TryCatch<'b, active::HandleScope<'c>>
{
  type NewScope = active::ContextScope<'a, active::HandleScope<'c>>;
}

// ===== HandleScope<'a> =====

impl<'a, 'b: 'a> AddHandleScope<'a> for active::ContextScope<'b> {
  type NewScope = active::HandleScope<'a>;
}

impl<'a, 'b: 'a, P: AddHandleScope<'a>> AddHandleScope<'a>
  for active::ContextScope<'b, P>
{
  type NewScope = <P as AddHandleScope<'a>>::NewScope;
}

impl<'a, 'b: 'a> AddHandleScope<'a> for active::HandleScope<'b> {
  type NewScope = active::HandleScope<'a>;
}

impl<'a, 'b: 'a, 'c: 'b> AddHandleScope<'a>
  for active::EscapableHandleScope<'b, 'c>
{
  type NewScope = active::EscapableHandleScope<'a, 'c>;
}

impl<'a, 'b: 'a, 'c: 'b, 'd: 'c> AddHandleScope<'a>
  for active::TryCatch<'b, active::EscapableHandleScope<'c, 'd>>
{
  type NewScope = active::EscapableHandleScope<'a, 'd>;
}

impl<'a, 'b: 'a, 'c: 'b> AddHandleScope<'a>
  for active::TryCatch<'b, active::HandleScope<'c>>
{
  type NewScope = active::HandleScope<'a>;
}

// ===== EscapableHandleScope<'a, 'b> =====

impl<'a, 'b: 'a, P: AddEscapableHandleScope<'a>> AddEscapableHandleScope<'a>
  for active::ContextScope<'b, P>
{
  type NewScope = <P as AddEscapableHandleScope<'a>>::NewScope;
}

impl<'a, 'b: 'a> AddEscapableHandleScope<'a> for active::HandleScope<'b> {
  type NewScope = active::EscapableHandleScope<'a, 'b>;
}

impl<'a, 'b: 'a, 'c: 'b> AddEscapableHandleScope<'a>
  for active::EscapableHandleScope<'b, 'c>
{
  type NewScope = active::EscapableHandleScope<'a, 'b>;
}

impl<'a, 'b: 'a, 'c: 'b, 'd: 'c> AddEscapableHandleScope<'a>
  for active::TryCatch<'b, active::EscapableHandleScope<'c, 'd>>
{
  type NewScope = active::EscapableHandleScope<'a, 'c>;
}

impl<'a, 'b: 'a, 'c: 'b> AddEscapableHandleScope<'a>
  for active::TryCatch<'b, active::HandleScope<'c>>
{
  type NewScope = active::EscapableHandleScope<'a, 'c>;
}

// ===== TryCatch<'a> =====

impl<'a, 'b: 'a, P: AddTryCatch<'a>> AddTryCatch<'a>
  for active::ContextScope<'b, P>
{
  type NewScope = <P as AddTryCatch<'a>>::NewScope;
}

impl<'a, 'b: 'a> AddTryCatch<'a> for active::HandleScope<'b> {
  type NewScope = active::TryCatch<'a, active::HandleScope<'b>>;
}

impl<'a, 'b: 'a, 'c: 'b> AddTryCatch<'a>
  for active::EscapableHandleScope<'b, 'c>
{
  type NewScope = active::TryCatch<'a, active::EscapableHandleScope<'b, 'c>>;
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
  pub struct TryCatch<'a, P = ()> {
    pub(super) effective_scope: NonNull<EffectiveScope>,
    _phantom: PhantomData<&'a mut P>,
  }

  impl<'a> ContextScope<'a> {
    pub fn root(
      context: &'a Context,
    ) -> data2::ContextScope<'a, active::ContextScope<'a, ()>> {
      data2::ContextScope::root(context)
    }
    pub fn new<'b: 'a, P: AddContextScope<'a> + 'b>(
      parent: &'a mut P,
      context: &'a Context,
    ) -> data2::ContextScope<'a, <P as AddContextScope<'a>>::NewScope> {
      data2::ContextScope::new(parent, context)
    }
  }

  impl<'a, P: RootHandleScopeParam<'a> + 'a> HandleScope<'a, P> {
    pub fn root<Q: Deref<Target = P>>(isolate_or_context: &'a Q) -> P::Data {
      P::root(isolate_or_context.deref())
    }
  }

  impl<'a> HandleScope<'a> {
    pub fn new<'b: 'a, P: AddHandleScope<'a> + 'b>(
      parent: &'a mut P,
    ) -> data2::HandleScope<'a, <P as AddHandleScope<'a>>::NewScope> {
      data2::HandleScope::new(parent)
    }
  }

  impl<'a, 'b> EscapableHandleScope<'a, 'b> {
    pub fn new<'c: 'a, P: AddEscapableHandleScope<'a> + 'c>(
      parent: &'a mut P,
    ) -> data2::EscapableHandleScope<
      'a,
      <P as AddEscapableHandleScope<'a>>::NewScope,
    > {
      data2::EscapableHandleScope::new(parent.get_effective_scope())
    }
  }

  impl<'a> TryCatch<'a> {
    pub fn new<'b: 'a, P: AddTryCatch<'a> + 'b>(
      parent: &'a mut P,
    ) -> data2::TryCatch<'a, <P as AddTryCatch<'a>>::NewScope> {
      data2::TryCatch::new(parent.get_effective_scope())
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

  impl<'a> HandleScope<'a, Isolate> {}
  impl<'a> HandleScope<'a, Context> {}
  impl<'a, 'b> EscapableHandleScope<'a, 'b> {}
  impl<'a, 'b, 'c> TryCatch<'a, EscapableHandleScope<'b, 'c>> {}
  impl<'a, 'b> TryCatch<'a, HandleScope<'b, Context>> {}
}

pub struct EffectiveScope {
  isolate: *mut Isolate,
  last_entered: Option<NonNull<NonNull<EffectiveScope>>>,
  context: Option<NonNull<Context>>,
  escape_slot: Option<NonNull<raw::Address>>,
  try_catch: Option<NonNull<raw::TryCatch>>,
}

mod data2 {
  use super::*;

  pub struct ContextScope<'a, E> {
    inner: ScopeDataInner<aspect::Context>,
    _phantom: PhantomData<&'a mut E>,
  }
  impl<'a, E> ContextScope<'a, E> {
    pub(super) fn root(context: &Context) -> Self {
      Self::new_impl(context.get_effective_scope(), context)
    }
    pub(super) fn new(
      parent: &'a mut impl GetEffectiveScope,
      context: &Context,
    ) -> Self {
      Self::new_impl(parent.get_effective_scope(), context)
    }
    fn new_impl(effective: NonNull<EffectiveScope>, context: &Context) -> Self {
      Self {
        inner: ScopeDataInner::new(
          effective,
          aspect::Context::new(context),
          (),
        ),
        _phantom: PhantomData,
      }
    }
    pub fn enter(&'a mut self) -> &'a mut E {
      self.inner.enter()
    }
  }

  pub struct HandleScope<'a, E> {
    inner: ScopeDataInner<aspect::HandleScope>,
    _phantom: PhantomData<&'a mut E>,
  }
  impl<'a> HandleScope<'a, active::HandleScope<'a, Isolate>> {
    pub(super) fn root(isolate: &'a Isolate) -> Self {
      Self::new_impl(isolate.get_effective_scope())
    }
  }
  impl<'a, E> HandleScope<'a, E> {
    pub(super) fn new(parent: &'a mut impl GetEffectiveScope) -> Self {
      Self::new_impl(parent.get_effective_scope())
    }
    fn new_impl(effective: NonNull<EffectiveScope>) -> Self {
      Self {
        inner: ScopeDataInner::new(effective, aspect::HandleScope::new(), ()),
        _phantom: PhantomData,
      }
    }
    pub fn enter(&'a mut self) -> &'a mut E {
      self.inner.enter()
    }
  }

  pub struct ContextAndHandleScope<'a, E> {
    inner: ScopeDataInner<aspect::Context, aspect::HandleScope>,
    _phantom: PhantomData<&'a mut E>,
  }
  impl<'a, E> ContextAndHandleScope<'a, E> {
    pub(super) fn root(context: &'a Context) -> Self {
      Self {
        inner: ScopeDataInner::new(
          context.get_effective_scope(),
          aspect::Context::new(context),
          aspect::HandleScope::new(),
        ),
        _phantom: PhantomData,
      }
    }
    pub fn enter(&'a mut self) -> &'a mut E {
      self.inner.enter()
    }
  }

  pub struct EscapableHandleScope<'a, E> {
    inner: ScopeDataInner<aspect::EscapeSlot, aspect::HandleScope>,
    _phantom: PhantomData<&'a mut E>,
  }
  impl<'a, E> EscapableHandleScope<'a, E> {
    pub(super) fn new(effective: NonNull<EffectiveScope>) -> Self {
      Self {
        inner: ScopeDataInner::new(
          effective,
          aspect::EscapeSlot::new(),
          aspect::HandleScope::new(),
        ),
        _phantom: PhantomData,
      }
    }
    pub fn enter(&'a mut self) -> &'a mut E {
      self.inner.enter()
    }
  }

  pub struct TryCatch<'a, E> {
    inner: ScopeDataInner<aspect::TryCatch>,
    _phantom: PhantomData<&'a mut E>,
  }
  impl<'a, E> TryCatch<'a, E> {
    pub(super) fn new(effective: NonNull<EffectiveScope>) -> Self {
      Self {
        inner: ScopeDataInner::new(effective, aspect::TryCatch::new(), ()),
        _phantom: PhantomData,
      }
    }
    pub fn enter(&'a mut self) -> &'a mut E {
      self.inner.enter()
    }
  }

  struct ScopeDataInner<A1: aspect::Aspect = (), A2: aspect::Aspect = ()> {
    header: Header,
    aspect1: A1,
    aspect2: A2,
  }

  impl<A1: aspect::Aspect, A2: aspect::Aspect> ScopeDataInner<A1, A2> {
    fn new(
      effective_scope: NonNull<EffectiveScope>,
      aspect1: A1,
      aspect2: A2,
    ) -> Self {
      Self {
        header: Header::new(effective_scope),
        aspect1,
        aspect2,
      }
    }

    fn enter<'a, E>(&'a mut self) -> &'a mut E {
      {
        let effective_scope = self.header.enter();
        self.aspect1.enter(effective_scope);
        self.aspect2.enter(effective_scope);
      }
      assert_type_layout_eq::<NonNull<EffectiveScope>, E>();
      let effective_scope_ref = self.header.get_effective_scope_ref();
      unsafe { &mut *(effective_scope_ref as *mut _ as *mut E) }
    }

    fn exit(&mut self) {
      let effective_scope = self.header.exit();
      // Exit in reverse order!
      self.aspect2.exit(effective_scope);
      self.aspect1.exit(effective_scope);
    }
  }

  impl<A1: aspect::Aspect, A2: aspect::Aspect> Drop for ScopeDataInner<A1, A2> {
    fn drop(&mut self) {
      if self.header.has_scope_been_entered() {
        self.exit()
      }
    }
  }

  #[derive(Eq, PartialEq)]
  pub struct Header {
    effective_scope: NonNull<EffectiveScope>,
    prior_effective_scope_ref: Option<NonNull<NonNull<EffectiveScope>>>,
  }

  impl Header {
    fn new(effective_scope: NonNull<EffectiveScope>) -> Self {
      // TODO: track parent for child scopes.
      Self {
        effective_scope,
        prior_effective_scope_ref: None,
      }
    }

    fn enter(&mut self) -> &mut EffectiveScope {
      let self_ref = NonNull::from(&self.effective_scope);
      let effective = unsafe { self.effective_scope.as_mut() };
      let prior_ref = effective.last_entered.replace(self_ref);
      match &mut self.prior_effective_scope_ref {
        p @ Some(_) => assert_eq!(*p, prior_ref),
        p @ None => *p = prior_ref,
      };
      effective
    }

    fn exit(&mut self) -> &mut EffectiveScope {
      let self_ref = NonNull::from(&self.effective_scope);
      let effective = unsafe { self.effective_scope.as_mut() };
      let exited_ref =
        replace(&mut effective.last_entered, self.prior_effective_scope_ref)
          .unwrap();
      assert_eq!(exited_ref, self_ref);
      effective
    }

    fn get_effective_scope_ref(&mut self) -> &mut NonNull<EffectiveScope> {
      &mut self.effective_scope
    }

    fn has_scope_been_entered(&mut self) -> bool {
      let self_ref = NonNull::from(&self.effective_scope);
      let effective = unsafe { self.effective_scope.as_mut() };
      match effective.last_entered {
        Some(p) if p == self_ref => true,
        p if p == self.prior_effective_scope_ref => false,
        _ => panic!("cannot use scope while it is shadowed"),
      }
    }
  }
}

pub(crate) mod aspect {
  use super::*;

  pub(super) trait Aspect {
    fn enter(&mut self, effective_scope: &mut EffectiveScope);
    fn exit(&mut self, effective_scope: &mut EffectiveScope);
  }

  impl Aspect for () {
    fn enter(&mut self, _effective_scope: &mut EffectiveScope) {}
    fn exit(&mut self, _effective_scope: &mut EffectiveScope) {}
  }

  #[repr(C)]
  pub struct Context {
    context: NonNull<super::Context>,
    prior: Option<NonNull<super::Context>>,
  }
  impl Context {
    pub fn new(context: &super::Context) -> Self {
      Self {
        context: NonNull::from(context),
        prior: None,
      }
    }
  }
  impl Aspect for Context {
    fn enter(&mut self, effective_scope: &mut EffectiveScope) {
      let prior = effective_scope.context.replace(self.context);
      let none = replace(&mut self.prior, prior);
      assert!(none.is_none());
      // TODO: do not enter/exit when context and prior are equal.
      unsafe { raw::v8__Context__Enter(self.context.as_ptr()) }
    }
    fn exit(&mut self, effective_scope: &mut EffectiveScope) {
      let prior = self.prior.take();
      let context = replace(&mut effective_scope.context, prior).unwrap();
      assert_eq!(context, self.context);
      // TODO: do not enter/exit when context and prior are equal.
      unsafe { raw::v8__Context__Exit(self.context.as_ptr()) }
    }
  }

  #[repr(C)]
  pub(super) struct HandleScope {
    raw: MaybeUninit<raw::HandleScope>,
  }
  impl HandleScope {
    pub fn new() -> Self {
      Self {
        raw: MaybeUninit::uninit(),
      }
    }
  }
  impl Aspect for HandleScope {
    fn enter(&mut self, effective_scope: &mut EffectiveScope) {
      let isolate = effective_scope.isolate;
      unsafe { raw::v8__HandleScope__CONSTRUCT(&mut self.raw, isolate) }
    }
    fn exit(&mut self, _effective_scope: &mut EffectiveScope) {
      unsafe { raw::v8__HandleScope__DESTRUCT(self.raw.as_mut_ptr()) }
    }
  }

  pub(super) struct EscapeSlot {
    prior: Option<NonNull<raw::Address>>,
  }
  impl EscapeSlot {
    pub fn new() -> Self {
      Self { prior: None }
    }
  }
  impl Aspect for EscapeSlot {
    fn enter(&mut self, effective_scope: &mut EffectiveScope) {
      let isolate = effective_scope.isolate;
      let undefined: &Data = unsafe { &*raw::v8__Undefined(isolate) };
      let slot: &Data = unsafe { &*raw::v8__Local__New(isolate, undefined) };
      let slot = NonNull::from(slot).cast::<raw::Address>();
      let prior = effective_scope.escape_slot.replace(slot);
      let none = replace(&mut self.prior, prior);
      assert!(none.is_none());
    }
    fn exit(&mut self, effective_scope: &mut EffectiveScope) {
      let prior = self.prior.take();
      replace(&mut effective_scope.escape_slot, prior).unwrap();
      // Note: an escape slot is essentially a mutable Local that exists in an
      // ancestor HandleScope. Like any other local it does not need need to
      // be cleaned up. Also note that `effective_scope.escape_slot` will turn
      // into `None` after it has been used.
    }
  }

  #[repr(C)]
  pub(super) struct TryCatch {
    raw: MaybeUninit<raw::TryCatch>,
    prior: Option<NonNull<raw::TryCatch>>,
  }
  impl TryCatch {
    pub fn new() -> Self {
      Self {
        raw: MaybeUninit::uninit(),
        prior: None,
      }
    }
  }
  impl Aspect for TryCatch {
    fn enter(&mut self, effective_scope: &mut EffectiveScope) {
      let isolate = effective_scope.isolate;
      let tc = unsafe {
        raw::v8__TryCatch__CONSTRUCT(&mut self.raw, isolate);
        NonNull::new_unchecked(self.raw.as_mut_ptr())
      };
      let prior = effective_scope.try_catch.replace(tc);
      let none = replace(&mut self.prior, prior);
      assert!(none.is_none());
    }
    fn exit(&mut self, effective_scope: &mut EffectiveScope) {
      let prior = self.prior.take();
      let tc = replace(&mut effective_scope.try_catch, prior);
      let tc = tc.unwrap().as_ptr();
      assert_eq!(tc, self.raw.as_mut_ptr());
      unsafe { raw::v8__TryCatch__DESTRUCT(tc) }
    }
  }
}

mod raw {
  use super::*;

  #[repr(C)]
  pub struct HandleScope([usize; 3]);

  #[derive(Clone, Copy)]
  #[repr(transparent)]
  pub struct Address(usize);

  #[repr(C)]
  pub struct TryCatch([usize; 6]);

  extern "C" {
    pub fn v8__Isolate__GetCurrentContext(
      isolate: *mut Isolate,
    ) -> *const Context;
    pub fn v8__Isolate__GetEnteredOrMicrotaskContext(
      isolate: *mut Isolate,
    ) -> *const Context;

    pub fn v8__Context__GetIsolate(this: *const Context) -> *mut Isolate;
    pub fn v8__Context__Enter(this: *const Context);
    pub fn v8__Context__Exit(this: *const Context);

    pub fn v8__HandleScope__CONSTRUCT(
      buf: *mut MaybeUninit<HandleScope>,
      isolate: *mut Isolate,
    );
    pub fn v8__HandleScope__DESTRUCT(this: *mut HandleScope);

    pub fn v8__Undefined(isolate: *mut Isolate) -> *const Primitive;
    pub fn v8__Local__New(
      isolate: *mut Isolate,
      other: *const Data,
    ) -> *const Data;

    pub fn v8__TryCatch__CONSTRUCT(
      buf: *mut MaybeUninit<TryCatch>,
      isolate: *mut Isolate,
    );
    pub fn v8__TryCatch__DESTRUCT(this: *mut TryCatch);
    pub fn v8__TryCatch__HasCaught(this: *const TryCatch) -> bool;
    pub fn v8__TryCatch__CanContinue(this: *const TryCatch) -> bool;
    pub fn v8__TryCatch__HasTerminated(this: *const TryCatch) -> bool;
    pub fn v8__TryCatch__Exception(this: *const TryCatch) -> *const Value;
    pub fn v8__TryCatch__StackTrace(
      this: *const TryCatch,
      context: *const Context,
    ) -> *const Value;
    pub fn v8__TryCatch__Message(this: *const TryCatch) -> *const Message;
    pub fn v8__TryCatch__Reset(this: *mut TryCatch);
    pub fn v8__TryCatch__ReThrow(this: *mut TryCatch) -> *const Value;
    pub fn v8__TryCatch__IsVerbose(this: *const TryCatch) -> bool;
    pub fn v8__TryCatch__SetVerbose(this: *mut TryCatch, value: bool);
    pub fn v8__TryCatch__SetCaptureMessage(this: *mut TryCatch, value: bool);
  }
}

mod raw_unused {
  use super::*;

  #[repr(C)]
  pub struct EscapableHandleScope([usize; 4]);

  extern "C" {
    fn v8__EscapableHandleScope__CONSTRUCT(
      buf: *mut MaybeUninit<EscapableHandleScope>,
      isolate: *mut Isolate,
    );
    fn v8__EscapableHandleScope__DESTRUCT(this: *mut EscapableHandleScope);
    fn v8__EscapableHandleScope__GetIsolate(
      this: &EscapableHandleScope,
    ) -> *mut Isolate;
    fn v8__EscapableHandleScope__Escape(
      this: *mut EscapableHandleScope,
      value: *const Data,
    ) -> *const Data;
  }
}
#[doc(inline)]
pub use active::*;

#[inline(always)]
fn assert_type_layout_eq<S, T>() {
  assert_eq!(size_of::<S>(), size_of::<T>());
  assert_eq!(align_of::<S>(), align_of::<T>());
}
