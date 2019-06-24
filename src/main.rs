#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]

extern crate owning_ref;

use core::cell::UnsafeCell;
use owning_ref::{OwningRef, StableAddress};
use std::cell::Cell;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;

struct Scope {}

struct VM<'a, P> {
  parent: Option<&'a mut P>,
  scope: Cell<Scope>,
}

impl<'a> VM<'a, ()> {
  fn new() -> Self {
    VM {
      parent: None,
      scope: Cell::new(Scope {}),
    }
  }
}

impl<'a, P> VM<'a, P> {
  fn enter_scope<'n>(&'n mut self) -> VM<'n, VM<'a, P>> {
    VM {
      parent: Some(self),
      scope: Cell::new(Scope {}),
    }
  }

  fn dispose(mut self) {}
}

struct Local<'sc> {
  val: i32,
  scope: PhantomData<&'sc Scope>,
}

impl<'sc> Local<'sc> {
  fn new<T>(e: &mut VM<'sc, T>) -> Self {
    Self {
      val: 0,
      scope: PhantomData,
    }
  }

  fn live(&self) {}
}

fn main() {
  let mut l3;

  let ref mut vm1 = VM::new();

  let mut l1a = Local::new(vm1);
  let mut l1b = Local::new(vm1);

  {
    let ref mut vm2 = vm1.enter_scope();

    let mut l2a = Local::new(vm2);
    let mut l2b = Local::new(vm2);

    let mut _vm = vm1.enter_scope(); // fail
    let mut _a = Local::new(vm1); // fail

    {
      let mut vm3 = vm2.enter_scope();
      l3 = Local::new(&mut vm3);

      let mut _l = Local::new(vm1); // fail
      let mut _l = Local::new(vm2); // fail

      vm3.dispose();

      let mut _l = Local::new(vm2); // fail

      l3.live();

      let mut l2c = Local::new(vm2);
    }

    Local::new(vm1); // fail
    l2a.live();
  }

  let mut l1c = Local::new(vm1);

  l1a.live();

  //l3.live(); // fail
}
