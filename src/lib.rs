#![allow(dead_code)]

use std::marker::PhantomData;
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

pub trait AddHandleScope<'a> {
  type NewScope;
}
pub trait AddEscapableHandleScope<'a> {
  type NewScope;
}
pub trait AddTryCatch<'a> {
  type NewScope;
}

// ===== HandleScope<'a> =====

impl<'a> AddHandleScope<'a> for Context {
  type NewScope = alloc::HandleScope<'a>;
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

impl<'a, 'b: 'a> AddTryCatch<'a> for active::HandleScope<'b> {
  type NewScope = alloc::TryCatch<'a, active::HandleScope<'b>>;
}

impl<'a, 'b: 'a, 'c: 'b> AddTryCatch<'a>
  for active::EscapableHandleScope<'b, 'c>
{
  type NewScope = alloc::TryCatch<'a, active::EscapableHandleScope<'b, 'c>>;
}

pub(self) mod data {
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
  impl Drop for EscapableHandleScope {
    fn drop(&mut self) {}
  }
  impl Drop for TryCatch {
    fn drop(&mut self) {}
  }
}

pub mod alloc {
  use super::*;
  pub enum HandleScope<'a, P = Context> {
    Declared(&'a mut P),
    Entered(data::HandleScope),
  }
  pub enum EscapableHandleScope<'a, 'b, P = Context> {
    Declared {
      parent: &'a mut P,
      escape_slot: active::EscapeSlot<'b>,
    },
    Entered(data::EscapableHandleScope),
  }
  pub enum TryCatch<'a, P = Context> {
    Declared(&'a mut P),
    Entered(data::HandleScope),
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

  pub struct EscapeSlot<'a>(*const (), PhantomData<&'a mut ()>);
  pub struct HandleScope<'a, P = Context> {
    effective_scope: NonNull<EffectiveScope>,
    _phantom: PhantomData<&'a mut P>,
  }
  pub struct EscapableHandleScope<'a, 'b> {
    effective_scope: EffectiveScope,
    _phantom: PhantomData<(&'a mut (), &'b mut ())>,
  }
  pub struct TryCatch<'a, P = Context> {
    effective_scope: EffectiveScope,
    _phantom: PhantomData<&'a mut P>,
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

  impl<'a, P> Drop for HandleScope<'a, P> {
    fn drop(&mut self) {}
  }
  impl<'a, 'b> Drop for EscapableHandleScope<'a, 'b> {
    fn drop(&mut self) {}
  }
  impl<'a, P> Drop for TryCatch<'a, P> {
    fn drop(&mut self) {}
  }

  impl<'a> Deref for HandleScope<'a> {
    type Target = HandleScope<'a, ()>;
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
    //try_catch: Option<NonNull<TryCatch>>,
  }

  trait Template {
    type Param;

    fn new(param: Self::Param) -> Self;

    // ? fn get_self_anchor(&mut self) -> NonNull<NonNull<EffectiveScope>>;
    // ? fn get_prior_anchor(&mut self) -> NonNull<NonNull<EffectiveScope>>;

    fn become_topmost(&mut self) {
      let eff = self.get_effective_scope();
      let prior = self.get_prior();
      let prev = eff.topmost_scope.replace(NonNull::from(eff));
      assert_eq!(prior, prev);
    }
    fn restore_topmost(&mut self) {}

    fn enter_context(&mut self) {}
    fn exit_context(&mut self) {}

    fn init_handle_scope(&mut self) {}
    fn drop_handle_scope(&mut self) {}
    fn init_escape_slot(&mut self) {}
    fn drop_escape_slot(&mut self) {}
    fn init_try_catch(&mut self) {}
    fn drop_try_catch(&mut self) {}

    fn enter(&mut self) {
      self.assert_prior_topmost();
      self.become_topmost();
      self.assert_self_topmost();

      self.enter_context();
      self.init_escape_slot();
      self.init_handle_scope();
      self.init_try_catch();
    }

    fn exit(&mut self) {
      self.assert_self_topmost();
      self.restore_topmost();
      self.assert_prior_topmost();

      self.drop_try_catch();
      self.drop_handle_scope();
      self.drop_escape_slot();
      self.exit_context();
    }
  }

  struct HandleScope<'a, P = ()> {
    prior: &'a mut NonNull<EffectiveScope>,
    _phantom: PhantomData<P>,
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
