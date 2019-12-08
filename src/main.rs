use std::fmt::Debug;
use std::marker::PhantomData;
use std::marker::Sized;

/// Trait that defines the Rust equivalent of a C++ RAII object. Note that these
/// object generally must not move (at least not without invoking the C++ move
/// constructor, which is not possible in Rust) and in certain cases they *must*
/// be allocated on the stack (TryCatch and BackupIncumbentScope).
pub trait RAII {
    fn acquire(&mut self);
    fn release(&mut self);
}

#[repr(C)]
pub union HandleScope1 {
    isolate: *mut Isolate,
    data: [usize; 3],
}

impl HandleScope1 {
    pub fn new(isolate: *mut Isolate) -> Self {
        Self { isolate }
    }
}

impl RAII for HandleScope1 {
    fn acquire(&mut self) {
        unsafe {
            let Self { isolate: _isolate } = *self;
        }
    }
    fn release(&mut self) {
        let isolate: *mut Isolate = std::ptr::null_mut();
        *self = Self { isolate };
    }
}
//==

struct Opaque([u8; 0]);

#[repr(C)]
pub struct Isolate(Opaque);
#[repr(C)]
pub struct Context(Opaque);

impl Context {
    pub unsafe fn get_isolate(&self) -> *mut Isolate {
        std::ptr::null_mut()
    }

    pub fn new() -> Self {
        Self(unsafe { std::mem::transmute([0; 0]) })
    }
}

impl Isolate {
    pub fn new() -> Self {
        Self(unsafe { std::mem::transmute([0; 0]) })
    }
}

mod scope {
    use super::*;
    use std::ptr::null;
    use std::ptr::null_mut;

    pub trait Scope
    where
        Self: Sized,
    {
        fn enter(&mut self) -> Guard<Self> {
            Guard::new(self)
        }
    }

    pub struct Guard<'a, T>(&'a T);

    impl<'a, T> Guard<'a, T> {
        fn new(scope: &'a T) -> Self {
            Self(scope)
        }

        fn inner(&self) -> &T {
            &self.0
        }
    }

    impl<'a, T> Drop for Guard<'a, T> {
        fn drop(&mut self) {}
    }

    pub struct Locker<'i> {
        _has_lock: bool,
        _top_level: bool,
        isolate: &'i Isolate,
    }

    impl<'i> Locker<'i> {
        pub fn new(isolate: &'i Isolate) -> Self {
            Self {
                _has_lock: true,
                _top_level: true,
                isolate,
            }
        }
    }

    impl<'i> Scope for Locker<'i> {}

    pub trait LockerImpl
    where
        Self: Sized,
    {
        fn locked_cxx_isolate(guard: &mut Guard<Self>) -> *mut Isolate;
    }

    impl<'i> LockerImpl for Locker<'i> {
        fn locked_cxx_isolate(guard: &mut Guard<Self>) -> *mut Isolate {
            guard.inner().isolate as *const _ as *mut Isolate
        }
    }

    pub struct HandleScope<'sc> {
        isolate: *mut Isolate,
        _prev_next: *mut Opaque,
        _prev_limit: *mut Opaque,
        _parent_scope: PhantomData<&'sc ()>,
    }

    impl<'sc> HandleScope<'sc> {
        pub fn new<L>(parent_scope: &'sc mut Guard<'_, L>) -> Self
        where
            L: LockerImpl,
        {
            Self {
                isolate: LockerImpl::locked_cxx_isolate(parent_scope),
                _prev_next: null_mut(),
                _prev_limit: null_mut(),
                _parent_scope: PhantomData,
            }
        }
    }

    impl<'sc> Scope for HandleScope<'sc> {}

    impl<'sc> LockerImpl for HandleScope<'sc> {
        fn locked_cxx_isolate(guard: &mut Guard<Self>) -> *mut Isolate {
            guard.inner().isolate as *const _ as *mut Isolate
        }
    }

    pub trait HandleScopeImpl: LockerImpl {}
    impl<'sc> HandleScopeImpl for HandleScope<'sc> {}

    pub struct ContextScope<'l, 'sc, P> {
        context: Local<'l, Context>,
        _parent_scope: PhantomData<&'sc P>,
    }

    impl<'l, 'sc, P> ContextScope<'l, 'sc, P>
    where
        P: LockerImpl,
    {
        pub fn new(_parent_scope: &'_ mut Guard<'sc, P>, context: Local<'l, Context>) -> Self {
            Self {
                context,
                _parent_scope: PhantomData,
            }
        }
    }

    impl<'l, 'sc, P> Scope for ContextScope<'l, 'sc, P> where P: LockerImpl {}

    impl<'l, 'sc, P> LockerImpl for ContextScope<'l, 'sc, P>
    where
        P: LockerImpl,
    {
        fn locked_cxx_isolate(guard: &mut Guard<Self>) -> *mut Isolate {
            unsafe { (*guard.inner().context.value).get_isolate() as *const _ as *mut Isolate }
        }
    }

    impl<'l, 'sc, P> HandleScopeImpl for ContextScope<'l, 'sc, P> where P: HandleScopeImpl {}

    pub struct Local<'sc, T> {
        value: *const T,
        _scope: PhantomData<&'sc ()>,
    }

    impl<'sc, T> Copy for Local<'sc, T> {}
    impl<'sc, T> Clone for Local<'sc, T> {
        fn clone(&self) -> Self {
            *self
        }
    }

    impl<'sc, T> Local<'sc, T> {
        pub fn new<SC>(_scope: &'_ mut Guard<'sc, SC>, _value: T) -> Self
        where
            SC: HandleScopeImpl,
        {
            Self {
                value: null(),
                _scope: PhantomData,
            }
        }

        pub fn print_mut(&mut self) {
            println!("local {:?}", self.value);
        }
        pub fn print(&self) {
            println!("local {:?}", self.value);
        }
    }
}
use scope::ContextScope;
use scope::HandleScope;
use scope::Local;
use scope::Locker;
use scope::Scope;

pub trait Varia {
    type A;
    type B;
    type C;
}

fn main() {
    let isolate = Isolate::new();
    let mut locker = Locker::new(&isolate);
    let mut lock_guard = locker.enter();
    let mut data = HandleScope::new(&mut lock_guard);
    let mut scope = data.enter();
    let l1 = Local::new(&mut scope, 1);
    let ctx = Local::new(&mut scope, Context::new());
    let mut l2 = Local::new(&mut scope, 2);
    {
        let mut data2 = HandleScope::new(&mut scope);
        //let mut data3 = HandleScope::new(&mut scope);
        //let k = scope.iii();
        //std::mem::drop(data2);
        //scope.iii();
        let mut scope2 = data2.enter();
        let mut data3 = HandleScope::new(&mut scope2);
        let mut scope3 = data3.enter();
        let l22 = Local::new(&mut scope3, 22);
        //let l11 = Local::new(&mut scope, 22);
        //drop(scope3);
        //drop(data2);
        // drop(scope2);
        l22.print();
        //l22.print_mut();
        /*
        let l3 = Local::new(&mut scope2, 3);
        let mut l4 = Local::new(&mut scope2, 4);
        l3.print();
        l4.print_mut();
        let mut scope3 = HandleScope::new(&mut scope2);
        let mut scope3 = scope3.enter();
        let mut l6 = Local::new(&mut scope3, 11);
        l6.print_mut();*/
        l1.print();
        l2.print_mut();

        let mut scope4a = ContextScope::new(&mut scope3, ctx);
        let mut scope4b = scope4a.enter();
        let mut l4a = Local::new(&mut scope4b, 22);
        drop(scope4b);
        l4a.print_mut();
    }
    let l5 = Local::new(&mut scope, 9);
    l5.print();

    use std::sync::Mutex;
    let m1 = Mutex::new(1);
    let m2 = Mutex::new(m1);
    let m3 = Mutex::new(m2);
    let m4 = Mutex::new(m3);
    let g1 = m4.lock().unwrap();
    let g2 = g1.lock().unwrap();
    let g3 = g2.lock().unwrap();
    let g4 = g3.lock().unwrap();
    println!("{:?}", *g4);
}
