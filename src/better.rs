#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]

extern crate owning_ref;

use core::cell::UnsafeCell;
use owning_ref::{OwningRef, StableAddress};
use std::cell::Cell;
use std::marker::PhantomData;
use std::ops::Deref;
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

  fn get_scope(&'a mut self) -> &'a Scope {
    unimplemented!();
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

  //let mut hs1 = HandleScope::new();
  let mut vm1 = VM::new(); //&mut hs1);

  let mut l1a = Local::new(&mut vm1);
  let mut l1b = Local::new(&mut vm1);

  {
    //let mut hs2 = HandleScope::new();
    let mut vm2 = vm1.enter_scope(); //&mut hs2);

    let mut l2a = Local::new(&mut vm2);
    let mut l2b = Local::new(&mut vm2);

    let mut _vm = vm1.enter_scope(); // fail
    let mut _a = Local::new(&mut vm1); // fail

    {
      let mut vm3 = vm2.enter_scope();
      l3 = Local::new(&mut vm3);

      let mut _l = Local::new(&mut vm1); // fail
      let mut _l = Local::new(&mut vm2); // fail

      vm3.dispose();

      let mut _l = Local::new(&mut vm2); // fail

      l3.live();

      let mut l2c = Local::new(&mut vm2);
    }

    Local::new(&mut vm1); // fail
    l2a.live();
  }

  let mut l1c = Local::new(&mut vm1);

  l1a.live();

  //l3.live(); // fail
}
