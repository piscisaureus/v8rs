use std::default::Default;
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

impl Isolate {
    pub fn new() -> Self {
        Self(unsafe { std::mem::transmute([0; 0]) })
    }
}

mod scope {
    use super::*;

    pub trait Scope
    where
        Self: Sized,
    {
        fn enter(&mut self) -> Guard<Self> {
            Guard::new(self)
        }
    }

    pub trait ScopeImpl
    where
        Self: Sized,
    {
        fn cxx_isolate_private(&self) -> *mut Isolate;
        fn cxx_isolate(guard: &mut Guard<'_, Self>) -> *mut Isolate {
            guard.0.cxx_isolate_private()
        }
    }

    pub struct Guard<'a, T>(&'a T);

    impl<'a, T> Guard<'a, T> {
        fn new(scope: &'a T) -> Self {
            Self(scope)
        }
    }

    pub trait GuardImpl {
        type Inner: ScopeImpl;
        fn inner(&self) -> &Self::Inner;
    }

    impl<'a, T> GuardImpl for Guard<'a, T>
    where
        T: ScopeImpl,
    {
        type Inner = T;
        fn inner(&self) -> &Self::Inner {
            &self.0
        }
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

    impl<'i> ScopeImpl for Locker<'i> {
        fn cxx_isolate_private(&self) -> *mut Isolate {
            self.isolate as *const _ as *mut Isolate
        }
    }

    pub trait LockerImpl: ScopeImpl {}
    impl<'i> LockerImpl for Locker<'i> {}

    pub trait LockerGuard: GuardImpl {}
    impl<'sc, L> LockerGuard for Guard<'sc, L> where L: LockerImpl {}

    pub struct HandleScope<'sc, G>
    where
        G: LockerGuard,
    {
        _data: [usize; 3],
        parent: &'sc mut G,
    }

    impl<'sc, G> HandleScope<'sc, G>
    where
        G: LockerGuard,
    {
        pub fn new(parent: &'sc mut G) -> Self {
            Self {
                parent,
                _data: Default::default(),
            }
        }
    }

    impl<'sc, G> Scope for HandleScope<'sc, G> where G: LockerGuard {}

    impl<'sc, G> ScopeImpl for HandleScope<'sc, G>
    where
        G: LockerGuard,
    {
        fn cxx_isolate_private(&self) -> *mut Isolate {
            self.parent.inner().cxx_isolate_private()
        }
    }
    impl<'sc, G> LockerImpl for HandleScope<'sc, G> where G: LockerGuard {}

    pub trait HandleScopeImpl: LockerImpl {}
    impl<'sc, G> HandleScopeImpl for HandleScope<'sc, G> where G: LockerGuard {}

    pub trait HandleScopeGuard<'sc> {}
    impl<'sc, S> HandleScopeGuard<'sc> for Guard<'sc, S> where S: HandleScopeImpl {}

    pub struct Local<'sc, T>
    where
        T: Copy + Debug,
    {
        value: T,
        scope: PhantomData<&'sc ()>,
    }

    impl<'sc, T> Local<'sc, T>
    where
        T: Copy + Debug,
    {
        pub fn new(_scope: &'_ mut impl HandleScopeGuard<'sc>, value: T) -> Self {
            Self {
                value,
                scope: PhantomData,
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
use scope::HandleScope;
use scope::Local;
use scope::Locker;
use scope::Scope;

fn main() {
    let isolate = Isolate::new();
    let mut locker = Locker::new(&isolate);
    let mut lock_guard = locker.enter();
    let mut data = HandleScope::new(&mut lock_guard);
    let mut scope = data.enter();
    let l1 = Local::new(&mut scope, 1);
    let mut l2 = Local::new(&mut scope, 2);
    {
        let mut data2 = HandleScope::new(&mut scope);
        let mut scope2 = data2.enter();
        let l3 = Local::new(&mut scope2, 3);
        let mut l4 = Local::new(&mut scope2, 4);
        l1.print();
        l2.print_mut();
        l3.print();
        l4.print_mut();
        let mut scope3 = HandleScope::new(&mut scope2);
        let mut scope3 = scope3.enter();
        let mut l6 = Local::new(&mut scope3, 11);
        l6.print_mut();
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
