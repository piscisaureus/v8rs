#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

extern crate owning_ref;

use std::marker::PhantomData;
use std::sync::Mutex;

struct Ref<'a, T>(&'a T);
struct RefMut<'a, T>(&'a mut T);

type ScopeInfo<'p, 'q, T> = (&'p T, &'q mut T);

fn make<T>() -> T {
  unimplemented!()
}

struct Scope<'p, T>(Ref<'p, T>, Ref<'p, T>);

impl<'p> Scope<'p, ()> {
  fn root() -> Self {
    unimplemented!()
  }
}

impl<'p, T> Scope<'p, T> {
  fn new<'a>((scope, lock): &'p mut (&'a T, &'a mut T)) -> Self {
    Self(Ref(scope), Ref(lock))
  }

  fn borrow<'a>(&'a mut self) -> (&'a Ref<'p, T>, &'a mut Ref<'p, T>) {
    (&self.0, &mut self.1)
  }

  //fn new<'a: 'p, 'p, T>((scope, lock): &'p mut (&'a T, &'a mut T)) -> (&'a Ref<'p, T>, &'a mut Ref<'p, T>) {
  //  make()
  //}

  //fn dispose<'a, 'b>(_: ScopeInfo<'a, 'b, T>) {}
  fn dispose(self) {}
}

fn dispose<T>(_: T) {}

struct Local<'sc, T> {
  val: i32,
  owner: &'sc T,
}

impl<'sc, T> Local<'sc, T> {
  fn new((scope, lock): &mut (&'sc T, &mut T)) -> Self {
    Self {
      val: 0,
      owner: *scope,
    }
  }

  fn new2((scope, lock): (&'sc T, &mut T)) -> Self {
    Self {
      val: 0,
      owner: *&scope,
    }
  }

  fn live(&self) {}
}

fn main() {
  //let mut l3;

  let mut s1_ = Scope::root();
  let mut s1 = s1_.borrow();

  let mut l1a = Local::new(&mut s1);
  let mut l1b = Local::new(&mut s1);

  {
    let mut s2_ = Scope::new(&mut s1);
    let mut s2 = s2_.borrow();

    let mut l2a = Local::new(&mut s2);
    let mut l2b = Local::new(&mut s2);

    let mut _a = Scope::new(&mut s1); // fail
    let mut _a = Local::new(&mut s1); // fail

    {
      let mut s3_ = Scope::new(&mut s2);

      let mut s3 = s3_.borrow();
      let mut l3 = Local::new(&mut s3);

      let mut _a = Local::new(&mut s1); // fail
      let mut _a = Local::new(&mut s2); // fail

      l3.live();
      s3_.dispose(); //Scope::dispose(s3);

      let mut l2c = Local::new(&mut s2);
    }

    s2_.dispose(); //Scope::dispose(s2); // Fail
    l2a.live();
  }

  let mut l1c = Local::new(&mut s1);

  l1a.live();

  s1_.dispose(); //Scope::dispose(s1); // fail
  l1a.live();

  //l3.live(); // fail
}
