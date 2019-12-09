use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::ptr::null_mut;

pub struct Opaque([u8; 0]);
pub struct Isolate(Opaque);

struct Guard<'sc, S>(&'sc S);

impl<'sc, S> Guard<'sc, S> {
    fn new(scope: &'sc mut S) -> Self {
        Self(scope)
    }
}

impl<'sc, S> Deref for Guard<'sc, S> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

trait Scope<'p, P>
where
    Self: Sized,
    P: 'p,
{
    type ParentScope;
    fn enter(&mut self) -> Guard<Self>;
}

#[repr(C)]
struct HandleScopeData {
    isolate: *mut Isolate,
    prev_next: *mut Opaque,
    prev_limit: *mut Opaque,
}

enum CxxScope<Args, Data> {
    Uninit(Args),
    Entered(Data),
    Empty(MaybeUninit<Data>),
}

struct HandleScope<'p, P>
where
    Self: Scope<'p, P>,
{
    cxx_scope: CxxScope<*mut Isolate, HandleScopeData>,
    parent_scope: <Self as Scope<'p, P>>::ParentScope,
}

impl<'p, P> HandleScope<'p, P> {
    fn new(parent: &'p mut Guard<'p, P>) -> Self {
        Self {
            cxx_scope: CxxScope::Uninit(null_mut()),
            parent_scope: PhantomData,
        }
    }
}

impl<'p, P> Scope<'p, P> for HandleScope<'p, P> {
    type ParentScope = PhantomData<&'p P>;
    fn enter(&mut self) -> Guard<Self> {
        if let CxxScope::Uninit(_) = self.cxx_scope {
            let u = std::mem::replace(
                &mut self.cxx_scope,
                CxxScope::Entered(unsafe { MaybeUninit::zeroed().assume_init() }),
            );
            let _args = match u {
                CxxScope::Uninit(args) => args,
                _ => unreachable!(),
            };
        }
        Guard::new(self)
    }
}

impl Scope<'static, ()> for () {
    type ParentScope = ();
    fn enter(&mut self) -> Guard<Self> {
        Guard::new(self)
    }
}

fn main() {
    let mut s0 = ();
    let mut g0 = s0.enter();

    let mut h1 = HandleScope::new(&mut g0);
    let mut g1 = h1.enter();

    let mut h2 = HandleScope::new(&mut g1);
    let mut g2 = h2.enter();

    std::mem::drop(h1);
}
