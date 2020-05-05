use derive_deref::*;
use std::any::Any;
use std::cell::Cell;
use std::cell::UnsafeCell;
use std::future::Future;
use std::marker::PhantomData;
use std::mem::transmute;
use std::mem::ManuallyDrop;
use std::mem::MaybeUninit;
use std::mem::*;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::ptr::NonNull;

#[derive(Default, Deref, DerefMut)]
struct DropMe<T>(T);
impl<T> Drop for DropMe<T> {
    fn drop(&mut self) {
        println!("dop!");
    }
}

#[derive(Default, Deref, DerefMut)]
struct DropMeGently<'a, T>(DropMe<T>, PhantomData<&'a ()>);
impl<'a, T> DropMeGently<'a, T> {
    fn new(val: T) -> Self {
        Self(DropMe(val), PhantomData)
    }
}

enum Store<'a, T, I: FnOnce() -> Scope<'a, T>> {
    Unentered(Option<ManuallyDrop<I>>),
    Entered(Scope<'a, T>),
    // _Phantom(PhantomData<&'a S>),
    _Phantom(DropMe<PhantomData<fn() -> I>>),
}

impl<'a, T, I: FnOnce() -> Scope<'a, T>> Store<'a, T, I> {
    fn enter(&'a mut self) -> &'a mut Scope<'a, T> {
        match self {
            Self::Unentered(f, ..) => {
                let f = ManuallyDrop::into_inner(f.take().unwrap());
                *self = Self::Entered(f());
            }
            Self::Entered(_) => {}
            _ => unreachable!(),
        };
        match self {
            Self::Unentered(..) => unreachable!(),
            Self::Entered(scope) => scope,
            _ => unreachable!(),
        }
    }
}

struct Scope<'a, T> {
    data: DropMe<T>,
    phantom_: PhantomData<&'a Self>,
}

impl<'a, T> Scope<'a, T> {
    fn new_root(val: T) -> Store<'a, T, impl FnOnce() -> Scope<'a, T>> {
        Store::Unentered(Some(ManuallyDrop::new(move || Self {
            data: DropMe(val),
            phantom_: PhantomData,
        })))
    }

    fn new<'p: 'a>(
        parent: &'a mut Scope<'p, T>,
        val: T,
    ) -> Store<'a, T, impl FnOnce() -> Scope<'a, T>> {
        Self::new_root(val)
    }

    pub fn new_local(self: &'_ mut Scope<'a, T>) -> Local<'a, T> {
        Local::new()
    }
}

struct Local<'a, T> {
    phantom_: PhantomData<&'a T>,
}
impl<'a, T> Local<'a, T> {
    fn new() -> Self {
        Self {
            phantom_: PhantomData,
        }
    }
}

fn use_it<T>(_: &T) {}

fn get() -> impl FnOnce() -> usize {
    || unimplemented!()
}

fn main() {
    let l0 = {
        let mut s00 = Scope::<u32>::new_root(1);
        let mut r00 = s00.enter();
        //
        let mut s0 = Scope::<u32>::new(&mut r00, 2);
        let mut r0 = s0.enter();
        let l0a = Scope::new_local(&mut r0);

        {
            let mut s1 = Scope::new(&mut r0, 3);
            {
                let mut r1 = s1.enter();
                let mut l1a = r1.new_local();
                let mut l1b = r1.new_local();
            }
        }
        let l0b = Scope::new_local(&mut r0);
        use_it(&l0a);
        l0a;
    };
}
