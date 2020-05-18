#![allow(dead_code)]

use std::marker::PhantomData as __;
use std::ptr::null_mut;

pub struct Context {
    ptr: *mut (),
}
impl Context {
    fn new() -> Self {
        Self { ptr: null_mut() }
    }
}

pub struct Local<'a, T> {
    ptr: *mut (),
    _phantom: __<&'a T>,
}
impl<'a, T> Local<'a, T> {
    pub fn new(_: &'_ mut HandleScope<'a>) -> Self {
        Self {
            ptr: null_mut(),
            _phantom: __,
        }
    }
}

pub trait DerivedScope<'a, P> {
    type NewScope;
}

impl<'a, 'b: 'a> DerivedScope<'a, HandleScope<'b, Context>> for HandleScope<'a> {
    type NewScope = alloc::HandleScope<'a, Context>;
}
impl<'a> DerivedScope<'a, Context> for HandleScope<'a> {
    type NewScope = alloc::HandleScope<'a, Context>;
}
impl<'a, 'b: 'a> DerivedScope<'a, HandleScope<'b, Context>> for TryCatch<'a> {
    type NewScope = alloc::TryCatch<'a, HandleScope<'b, Context>>;
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
            escape_sclot: active::EscapeSlot<'b>,
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
    impl<'a, 'b> EscapableHandleScope<'a, 'b, Context> {
        pub fn enter(&'a mut self) -> &'a mut active::EscapableHandleScope<'a, 'b, Context> {
            unimplemented!()
        }
    }
    impl<'a, 'b, 'c> TryCatch<'a, EscapableHandleScope<'b, 'c, Context>> {
        pub fn enter(&'a mut self) -> &'a mut TryCatch<'a, EscapableHandleScope<'b, 'c, Context>> {
            unimplemented!()
        }
    }
    impl<'a, 'b> TryCatch<'a, HandleScope<'b, Context>> {
        pub fn enter(&'a mut self) -> &'a mut TryCatch<'a, HandleScope<'b, Context>> {
            unimplemented!()
        }
    }
}

pub(self) mod active {
    use super::*;

    struct Common {
        isolate: *mut (),
    }

    pub struct EscapeSlot<'a>(*const (), __<&'a mut ()>);
    pub struct HandleScope<'a, P = Context> {
        common: Common,
        _phantom: __<&'a mut P>,
    }
    pub struct EscapableHandleScope<'a, 'b, P = Context> {
        common: Common,
        _phantom: __<(&'a mut P, &'b mut P)>,
    }
    pub struct TryCatch<'a, P = Context> {
        common: Common,
        _phantom: __<&'a mut P>,
    }

    impl<'a> HandleScope<'a> {
        pub fn root() -> alloc::HandleScope<'a, ()> {
            unimplemented!()
        }
        pub fn new<'b: 'a, P: 'b>(_parent: &'a mut P) -> <Self as DerivedScope<P>>::NewScope
        where
            Self: DerivedScope<'a, P>,
        {
            unimplemented!()
        }
    }
    impl<'a, 'b> EscapableHandleScope<'a, 'b> {
        pub fn new<'c: 'a, P: 'c>(_parent: &'a mut P) -> <Self as DerivedScope<P>>::NewScope
        where
            Self: DerivedScope<'a, P>,
        {
            unimplemented!()
        }
    }
    impl<'a> TryCatch<'a> {
        pub fn new<'b: 'a, P: 'b>(_parent: &'a mut P) -> <Self as DerivedScope<P>>::NewScope
        where
            Self: DerivedScope<'a, P>,
        {
            unimplemented!()
        }
    }

    impl<'a, P> Drop for HandleScope<'a, P> {
        fn drop(&mut self) {}
    }
    impl<'a, 'b, P> Drop for EscapableHandleScope<'a, 'b, P> {
        fn drop(&mut self) {}
    }
    impl<'a, P> Drop for TryCatch<'a, P> {
        fn drop(&mut self) {}
    }

    impl<'a> HandleScope<'a, ()> {}
    impl<'a> HandleScope<'a, Context> {}
    impl<'a, 'b> EscapableHandleScope<'a, 'b, Context> {}
    impl<'a, 'b, 'c> TryCatch<'a, EscapableHandleScope<'b, 'c, Context>> {}
    impl<'a, 'b> TryCatch<'a, HandleScope<'b, Context>> {}
}

use active::*;

fn main() {
    let mut root = HandleScope::root();
    let _root = root.enter();

    let mut ctx = Context::new();

    let mut s1 = HandleScope::new(&mut ctx);
    let s1 = s1.enter();

    let _s1l1 = Local::<i8>::new(s1);
    let _s1l2 = Local::<i8>::new(s1);
    let _fail = {
        let mut s2 = HandleScope::new(s1);
        let s2 = s2.enter();

        let s2l1 = Local::<i8>::new(s2);
        let _s2l2 = Local::<i8>::new(s2);
        //let _fail = Local::<i8>::new(s1);
        s2l1;
    };
    let _s1l3 = Local::<i8>::new(s1);
}
