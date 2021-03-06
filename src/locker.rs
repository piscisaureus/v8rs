// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
use crate::isolate::Isolate;
use std::marker::PhantomData;
use std::mem::MaybeUninit;

extern "C" {
  fn v8__Locker__CONSTRUCT(buf: &mut MaybeUninit<Locker>, isolate: &Isolate);
  fn v8__Locker__DESTRUCT(this: &mut Locker);
}

#[repr(C)]
/// v8::Locker is a scoped lock object. While it's active, i.e. between its
/// construction and destruction, the current thread is allowed to use the locked
/// isolate. V8 guarantees that an isolate can be locked by at most one thread at
/// any time. In other words, the scope of a v8::Locker is a critical section.
pub struct Locker<'sc>([usize; 2], PhantomData<&'sc mut ()>);

impl<'a> Locker<'a> {
  /// Initialize Locker for a given Isolate.
  pub fn new(isolate: &Isolate) -> Self {
    let mut buf = MaybeUninit::<Self>::uninit();
    unsafe {
      v8__Locker__CONSTRUCT(&mut buf, isolate);
      buf.assume_init()
    }
  }
}

impl<'a> Drop for Locker<'a> {
  fn drop(&mut self) {
    unsafe { v8__Locker__DESTRUCT(self) }
  }
}
