use std::marker::PhantomData;

struct Isolate(*mut [u8; 0]);

struct ScopeData<'p, D, P> {
    data: D,
    parent: &'p P,
}

impl<'p, D, P> ScopeData<'p, D, P> {
    pub fn new(guard: &'p mut Guard<'_, P>, data: D) -> Self {
        Self {
            data,
            parent: guard.inner(),
        }
    }
}

impl<'p> ScopeData<'p, Isolate, None> {
    pub fn new_root(data: Isolate) -> Self {
        Self {
            data,
            parent: &None,
        }
    }
}

fn get_inner<'d, 'p, D, P>(s: &'d ScopeData<'p, D, P>) -> &'d D {
    &s.data
}

impl<'p, D, P> std::ops::Deref for ScopeData<'p, D, P> {
    type Target = P;
    fn deref(&self) -> &Self::Target {
        &self.parent
    }
}

impl Isolate {
    pub fn new<'p>() -> ScopeData<'p, Self, None> {
        ScopeData::new_root(Self(std::ptr::null_mut()))
    }
}

struct Locker {}
impl Locker {
    pub fn new<'p, P>(guard: &'p mut Guard<'_, P>) -> ScopeData<'p, Self, P> {
        ScopeData::new(guard, Self {})
    }
}

struct Unlocker {}
impl Unlocker {
    pub fn new<'p, P: HasLock>(guard: &'p mut Guard<'_, P>) -> ScopeData<'p, Self, P> {
        ScopeData::new(guard, Self {})
    }
}

struct HandleScope {}
impl HandleScope {
    pub fn new<'p, P: HasLock>(guard: &'p mut Guard<'_, P>) -> ScopeData<'p, Self, P> {
        ScopeData::new(guard, Self {})
    }
}

struct EscapableHandleScope {}
impl EscapableHandleScope {
    pub fn new<'p, P: HasLock>(guard: &'p mut Guard<'_, P>) -> ScopeData<'p, Self, P> {
        ScopeData::new(guard, Self {})
    }
}

struct SealHandleScope {}
impl SealHandleScope {
    pub fn new<'p, P: HasLock>(guard: &'p mut Guard<'_, P>) -> ScopeData<'p, Self, P> {
        ScopeData::new(guard, Self {})
    }
}

struct TryCatch {}
impl TryCatch {
    pub fn new<'p, P: HasLock>(guard: &'p mut Guard<'_, P>) -> ScopeData<'p, Self, P> {
        ScopeData::new(guard, Self {})
    }
}

struct ContextScope {}
impl ContextScope {
    pub fn new<'p, P: HasLock>(guard: &'p mut Guard<'_, P>) -> ScopeData<'p, Self, P> {
        ScopeData::new(guard, Self {})
    }
}

struct Guard<'s, S>(&'s S);

impl<'s, S> Guard<'s, S> {
    pub fn new(scope: &'s S) -> Self {
        Self(scope)
    }

    pub fn inner(&self) -> &S {
        self.0
    }
}

struct Local<'sc, T> {
    value: *const T,
    _scope: PhantomData<&'sc ()>,
}

impl<'sc, T> Local<'sc, T> {
    pub fn new(_scope: &'_ mut impl HandleScopeGuard<'sc>, _value: T) -> Self {
        Self {
            value: std::ptr::null(),
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

struct None;

trait Scope
where
    Self: Sized,
{
    type Locker;
    type HandleScope;
    type TryCatch;
    type ContextScope;

    fn enter(&mut self) -> Guard<Self> {
        Guard::new(self)
    }
}

/*
trait Get<T> {
    fn get<'t, 's: 't>(scope: &'s Self) -> &'t T;
}

impl<T> Get<T> for T {
    fn get<'t, 's: 't>(scope: &'s Self) -> &'t T {
        scope
    }
}

impl<T, S> Get<T> for S
where
    Self: Scope,
    Self::Parent: Get<T>,
{
    fn get<'t, 's: 't>(scope: &'s Self) -> &'t T {
        guard.inner()
    }
}
*/

impl<'p> Scope for ScopeData<'p, Isolate, None> {
    type Locker = None;
    type HandleScope = None;
    type TryCatch = None;
    type ContextScope = None;
}

impl<'p, P> Scope for ScopeData<'p, Locker, P>
where
    P: Scope + 'p,
{
    type Locker = Self;
    type HandleScope = P::HandleScope;
    type TryCatch = P::TryCatch;
    type ContextScope = P::ContextScope;
}

impl<'p, P> Scope for ScopeData<'p, Unlocker, P>
where
    P: Scope + HasLock + 'p,
{
    type Locker = Self;
    type HandleScope = P::HandleScope;
    type TryCatch = P::TryCatch;
    type ContextScope = P::ContextScope;
}

impl<'p, P> Scope for ScopeData<'p, HandleScope, P>
where
    P: Scope + 'p,
{
    type Locker = P::Locker;
    type HandleScope = Self;
    type TryCatch = P::TryCatch;
    type ContextScope = P::ContextScope;
}

impl<'p, P> Scope for ScopeData<'p, EscapableHandleScope, P>
where
    P: Scope + 'p,
{
    type Locker = P::Locker;
    type HandleScope = Self;
    type TryCatch = P::TryCatch;
    type ContextScope = P::ContextScope;
}

impl<'p, P> Scope for ScopeData<'p, SealHandleScope, P>
where
    P: Scope + 'p,
{
    type Locker = P::Locker;
    type HandleScope = Self;
    type TryCatch = P::TryCatch;
    type ContextScope = P::ContextScope;
}

impl<'p, P> Scope for ScopeData<'p, TryCatch, P>
where
    P: Scope + 'p,
{
    type Locker = P::Locker;
    type HandleScope = Self;
    type TryCatch = P::TryCatch;
    type ContextScope = P::ContextScope;
}

impl<'p, P> Scope for ScopeData<'p, ContextScope, P>
where
    P: Scope + 'p,
{
    type Locker = P::Locker;
    type HandleScope = Self;
    type TryCatch = P::TryCatch;
    type ContextScope = P::ContextScope;
}

trait LockGuard<'s> {}
impl<'s, S> LockGuard<'s> for Guard<'s, S> where S: Scope + HasLock + 's {}

trait HandleScopeGuard<'s> {}
impl<'s, S> HandleScopeGuard<'s> for Guard<'s, S> where S: Scope + HasLock + HasHandles + 's {}

//impl<'s, S> HasContext for Guard<'s, S> where S: Scope + HasLock + HasContext {}
//impl<'s, S> HasTryCatch for Guard<'s, S> where S: Scope + HasLock + HasHandles + HasTryCatch {}

trait HasLock: Scope {}
trait HasHandles: Scope {}
trait HasContext: Scope {}
trait HasTryCatch: Scope {}

impl<'p, P: Scope + 'p> HasLock for ScopeData<'p, Locker, P> {}
impl<'p1, 'p2, P2: Scope> HasHandles for ScopeData<'p1, Locker, ScopeData<'p2, Unlocker, P2>> where
    P2: HasLock + HasHandles
{
}
impl<'p1, 'p2, P2: Scope> HasContext for ScopeData<'p1, Locker, ScopeData<'p2, Unlocker, P2>> where
    P2: HasLock + HasContext
{
}
impl<'p1, 'p2: 'p1, P2: Scope> HasTryCatch for ScopeData<'p1, Locker, ScopeData<'p2, Unlocker, P2>> where
    P2: HasLock + HasTryCatch + 'p2
{
}

impl<'p, P: Scope + 'p> HasLock for ScopeData<'p, HandleScope, P> where P: HasLock {}
impl<'p, P: Scope + 'p> HasHandles for ScopeData<'p, HandleScope, P> {}
impl<'p, P: Scope + 'p> HasContext for ScopeData<'p, HandleScope, P> where P: HasContext {}
impl<'p, P: Scope + 'p> HasTryCatch for ScopeData<'p, HandleScope, P> where P: HasTryCatch {}

impl<'p, P: Scope + 'p> HasLock for ScopeData<'p, EscapableHandleScope, P> where P: HasLock {}
impl<'p, P: Scope + 'p> HasHandles for ScopeData<'p, EscapableHandleScope, P> {}
impl<'p, P: Scope + 'p> HasContext for ScopeData<'p, EscapableHandleScope, P> where P: HasContext {}
impl<'p, P: Scope + 'p> HasTryCatch for ScopeData<'p, EscapableHandleScope, P> where P: HasTryCatch {}

impl<'p, P: Scope + 'p> HasLock for ScopeData<'p, SealHandleScope, P> where P: HasLock {}
impl<'p, P: Scope + 'p> HasContext for ScopeData<'p, SealHandleScope, P> where P: HasContext {}
impl<'p, P: Scope + 'p> HasTryCatch for ScopeData<'p, SealHandleScope, P> where P: HasTryCatch {}

impl<'p, P: Scope + 'p> HasLock for ScopeData<'p, ContextScope, P> where P: HasLock {}
impl<'p, P: Scope + 'p> HasHandles for ScopeData<'p, ContextScope, P> where P: HasHandles {}
impl<'p, P: Scope + 'p> HasContext for ScopeData<'p, ContextScope, P> {}
impl<'p, P: Scope + 'p> HasTryCatch for ScopeData<'p, ContextScope, P> where P: HasTryCatch {}

impl<'p, P: Scope + 'p> HasLock for ScopeData<'p, TryCatch, P> where P: HasLock {}
impl<'p, P: Scope + 'p> HasHandles for ScopeData<'p, TryCatch, P> where P: HasHandles {}
impl<'p, P: Scope + 'p> HasContext for ScopeData<'p, TryCatch, P> where P: HasContext {}
impl<'p, P: Scope + 'p> HasTryCatch for ScopeData<'p, TryCatch, P> {}

fn main() {
    println!("Hello, world!");

    let mut isolate = Isolate::new();
    let mut g = isolate.enter();

    let mut locker = Locker::new(&mut g);
    let mut g = locker.enter();

    // let mut unlocker = Unlocker::new(&mut g);
    // let mut g1 = unlocker.enter();

    let mut hs1 = HandleScope::new(&mut g);
    let mut g1 = hs1.enter();
    let mut l1 = Local::new(&mut g1, 1);
    let mut l2 = Local::new(&mut g1, "a");

    let mut hs2 = HandleScope::new(&mut g1);
    let i = get_inner::<Isolate, _>(&hs2);

    let mut g2 = hs2.enter();

    let mut l4 = Local::new(&mut g2, "a");
    let mut l3 = Local::new(&mut g2, 1);

    l1.print_mut();
    l2.print_mut();
    l3.print_mut();
    l4.print_mut();

    std::mem::drop(locker);
}
