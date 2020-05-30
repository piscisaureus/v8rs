#![allow(non_snake_case)]

use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::null;
use std::ptr::NonNull;

use crate::raw;
use crate::EffectiveScope as IsolateAnnex;
use crate::HandleScope;

#[repr(C)]
struct Opaque([u8; 1]);

#[repr(C)]
pub struct Isolate(Opaque);
impl Isolate {
  pub fn new() -> Box<Self> {
    Box::new(Self(unsafe { MaybeUninit::zeroed().assume_init() }))
  }
  pub(crate) fn get_annex(&self) -> &IsolateAnnex {
    unsafe { &mut *dangling_mut() }
  }
}

#[repr(C)]
pub struct Context(Opaque);
#[repr(C)]
pub struct Primitive(Opaque);
#[repr(C)]
pub struct Data(Opaque);
#[repr(C)]
pub struct Value(Opaque);
#[repr(C)]
pub struct Message(Opaque);
#[repr(C)]
pub struct Integer(Opaque);

impl Context {
  pub fn new<'a, P>(_scope: &'_ mut HandleScope<'a, P>) -> Local<'a, Self> {
    Local {
      raw: NonNull::dangling(),
      _phantom: PhantomData,
    }
  }
}

impl Deref for Primitive {
  type Target = Data;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const Self as *const Self::Target) }
  }
}

impl Integer {
  pub fn new<'a>(
    scope: &'_ mut HandleScope<'a>,
    _value: i32,
  ) -> Local<'a, Self> {
    Local::new(scope, dangling()).unwrap()
  }
}

pub struct Local<'a, T> {
  raw: NonNull<T>,
  _phantom: PhantomData<&'a T>,
}

impl<'a, T> Local<'a, T> {
  pub fn new(_: &'_ mut HandleScope<'a>, raw: *const T) -> Option<Self> {
    NonNull::new(raw as *mut T).map(|raw| Self {
      raw,
      _phantom: PhantomData,
    })
  }
}

impl<'a, T> Deref for Local<'a, T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    unsafe { &*self.raw.as_ptr() }
  }
}

pub extern "C" fn v8__Isolate__GetCurrentContext(
  isolate: *mut Isolate,
) -> *const Context {
  dangling()
}
pub extern "C" fn v8__Isolate__GetEnteredOrMicrotaskContext(
  isolate: *mut Isolate,
) -> *const Context {
  dangling()
}

fn v8__Context__GetIsolate(this: *const Context) -> *mut Isolate {
  dangling_mut()
}
fn v8__Context__Enter(this: *const Context) {}
fn v8__Context__Exit(this: *const Context) {}

pub extern "C" fn v8__HandleScope__CONSTRUCT(
  buf: *mut MaybeUninit<raw::HandleScope>,
  isolate: *mut Isolate,
) {
}
pub extern "C" fn v8__HandleScope__DESTRUCT(this: *mut raw::HandleScope) {}

pub extern "C" fn v8__Undefined(isolate: *mut Isolate) -> *const Primitive {
  dangling()
}
pub extern "C" fn v8__Local__New(
  isolate: *mut Isolate,
  other: *const Data,
) -> *const Data {
  dangling()
}

pub extern "C" fn v8__TryCatch__CONSTRUCT(
  buf: *mut MaybeUninit<raw::TryCatch>,
  isolate: *mut Isolate,
) {
}
pub extern "C" fn v8__TryCatch__DESTRUCT(this: *mut raw::TryCatch) {}
pub extern "C" fn v8__TryCatch__HasCaught(this: *const raw::TryCatch) -> bool {
  Default::default()
}
pub extern "C" fn v8__TryCatch__CanContinue(
  this: *const raw::TryCatch,
) -> bool {
  Default::default()
}
pub extern "C" fn v8__TryCatch__HasTerminated(
  this: *const raw::TryCatch,
) -> bool {
  Default::default()
}
pub extern "C" fn v8__TryCatch__Exception(
  this: *const raw::TryCatch,
) -> *const Value {
  dangling()
}
pub extern "C" fn v8__TryCatch__StackTrace(
  this: *const raw::TryCatch,
  context: *const Context,
) -> *const Value {
  dangling()
}
pub extern "C" fn v8__TryCatch__Message(
  this: *const raw::TryCatch,
) -> *const Message {
  dangling()
}
pub extern "C" fn v8__TryCatch__Reset(this: *mut raw::TryCatch) {}
pub extern "C" fn v8__TryCatch__ReThrow(
  this: *mut raw::TryCatch,
) -> *const Value {
  dangling()
}
pub extern "C" fn v8__TryCatch__IsVerbose(this: *const raw::TryCatch) -> bool {
  Default::default()
}
pub extern "C" fn v8__TryCatch__SetVerbose(
  this: *mut raw::TryCatch,
  value: bool,
) {
}
pub extern "C" fn v8__TryCatch__SetCaptureMessage(
  this: *mut raw::TryCatch,
  value: bool,
) {
}

fn dangling<T>() -> *const T {
  NonNull::dangling().as_ptr()
}

fn dangling_mut<T>() -> *mut T {
  NonNull::dangling().as_ptr()
}
