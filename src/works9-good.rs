#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]

use std::marker::PhantomData;

struct HandleScope {}

impl HandleScope {
  fn new() -> Self {
    Self {}
  }
  fn dispose(self) {}
}

struct VM<'a, P>(&'a HandleScope, Option<&'a mut P>);

impl<'a, P> VM<'a, P> {
  fn dispose(mut self) {}
}

impl<'a> VM<'a, ()> {
  fn new(storage: &'a mut HandleScope) -> Self {
    VM(storage, None)
  }
}

impl<'a, P> VM<'a, P> {
  fn enter<'n>(&'n mut self, storage: &'n mut HandleScope) -> VM<'n, VM<'a, P>> {
    VM(storage, Some(self))
  }
}

struct Local<'sc> {
  val: i32,
  handle_scope: PhantomData<&'sc HandleScope>,
}

impl<'sc> Local<'sc> {
  fn new<T>(e: &mut VM<'sc, T>) -> Self {
    Self {
      val: 0,
      handle_scope: PhantomData,
    }
  }

  fn live(&self) {}
}

fn main() {
  let mut l3;

  let mut hs1 = HandleScope::new();
  let mut vm1 = VM::new(&mut hs1);

  let mut l1a = Local::new(&mut vm1);
  let mut l1b = Local::new(&mut vm1);

  {
    let mut hs2 = HandleScope::new();
    let mut vm2 = vm1.enter(&mut hs2);

    let mut l2a = Local::new(&mut vm2);
    let mut l2b = Local::new(&mut vm2);

    let mut _hs = HandleScope::new();
    let mut _vm = vm1.enter(&mut _hs); // fail
    let mut _a = Local::new(&mut vm1); // fail

    {
      let mut hs3 = HandleScope {};
      let mut vm3 = vm2.enter(&mut hs3);
      l3 = Local::new(&mut vm3);

      let mut _l = Local::new(&mut vm1); // fail
      let mut _l = Local::new(&mut vm2); // fail

      vm3.dispose();
      hs3.dispose(); // Fail
      l3.live();

      let mut l2c = Local::new(&mut vm2);
    }

    hs2.dispose(); // Fail
    Local::new(&mut vm1); // fail
    l2a.live();
  }

  let mut l1c = Local::new(&mut vm1);

  l1a.live();

  hs1.dispose(); // Fail
  l1a.live();

  //l3.live(); // fail
}
