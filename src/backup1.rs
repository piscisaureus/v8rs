#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

use std::marker::PhantomData;
use std::sync::Mutex;

struct Ref<'a, T>(&'a T);
struct RefMut<'a, T>(&'a mut T);

struct Scope<'p, 'q, T>(pub &'p T, pub &'q mut T);

impl<'p> Scope<'p, 'p, ()> {
  fn root() -> Self {
    unimplemented!();
    //Self(&Ref(&()), &mut Ref(&()))
  }
}

impl<'p, T> Scope<'p, 'p, T> {
  //fn new<'a: 'p>((scope, lock): &'p mut (&'a T, &'a mut T)) -> Self {
  //  Self(Ref(scope), Ref(lock))
  //}
  //
  //fn borrow<'a: 'p>(&'a mut self) -> (&'a Ref<'p, T>, &'a mut Ref<'p, T>) {
  //  (&self.0, &mut self.1)
  //}

  //fn new<'b: 'p>((scope, lock): &'p mut (&'b T, &'b mut T)) -> Self {
  fn new<'a: 'p>(Scope(scope, lock): &'p mut Scope<'a, 'a, T>) -> Self {
    unimplemented!()
  }

  fn dispose(self) {}
}

struct Local<'sc, T> {
  val: i32,
  owner: &'sc T,
}

impl<'sc, T> Local<'sc, T> {
  //fn new((scope, lock): &'sc mut (&'sc T, &mut T)) -> Self {
  fn new(Scope(scope, lock): &mut Scope<'sc, 'sc, T>) -> Self {
    Self {
      val: 0,
      owner: *scope,
    }
  }

  fn live(&self) {}
}

fn main() {
  let mut l3;

  let mut s1 = Scope::root();

  let mut l1a = Local::new(&mut s1);
  let mut l1a = Local::new(&mut s1);

  {
    let mut s2 = Scope::new(&mut s1);

    let mut l2a = Local::new(&mut s2);
    let mut l2b = Local::new(&mut s2);

    let mut _a = Scope::new(&mut s1); // fail
    let mut _a = Local::new(&mut s1); // fail

    {
      let mut s3 = Scope::new(&mut s2);
      l3 = Local::new(&mut s3);

      let mut _a = Local::new(&mut s1); // fail
      let mut _a = Local::new(&mut s2); // fail

      l3.live();
      s3.dispose();

      let mut l2c = Local::new(&mut s2);
    }

    s2.dispose(); // Fail
    l2a.live();
  }

  let mut l1c = Local::new(&mut s1);

  l1a.live();

  s1.dispose(); // fail
  l1a.live();

  //l3.live(); // fail
}
