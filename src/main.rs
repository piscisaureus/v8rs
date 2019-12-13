use std::convert::Into;
use std::marker::PhantomData;

struct Isolate(*mut [u8; 0]);

impl Isolate {
    pub fn new() -> Self {
        Self(std::ptr::null_mut())
    }
}

struct Locker<'p, P>
where
    Self: Scope,
{
    parent: &'p <Self as Scope>::Parent,
}
impl<'p, P> Locker<'p, P>
where
    P: Scope + Into<<Self as Scope>::Parent>,
{
    fn new(guard: &'p mut Guard<'_, P>) -> Self {
        Self {
            parent: guard.inner().into(),
        }
    }
}

struct Unlocker<'p, P>
where
    Self: Scope,
{
    parent: &'p <Self as Scope>::Parent,
}
impl<'p, P> Unlocker<'p, P>
where
    P: Scope + HasLock + Into<<Self as Scope>::Parent>,
{
    fn new(guard: &'p mut Guard<'_, P>) -> Self {
        Self {
            parent: guard.inner().into(),
        }
    }
}

struct HandleScope<'p, P>
where
    Self: Scope,
{
    parent: &'p <Self as Scope>::Parent,
}
impl<'p, P> HandleScope<'p, P>
where
    P: Scope + HasLock + Into<<Self as Scope>::Parent>,
{
    fn new(guard: &'p mut Guard<'_, P>) -> Self {
        Self {
            parent: guard.inner().into(),
        }
    }
}

struct EscapableHandleScope<'p, P>
where
    Self: Scope,
{
    parent: &'p <Self as Scope>::Parent,
}
impl<'p, P> EscapableHandleScope<'p, P>
where
    P: Scope + HasLock + Into<<Self as Scope>::Parent>,
{
    fn new(guard: &'p mut Guard<'_, P>) -> Self {
        Self {
            parent: guard.inner().into(),
        }
    }
}

struct SealHandleScope<'p, P>
where
    Self: Scope,
{
    parent: &'p <Self as Scope>::Parent,
}
impl<'p, P> SealHandleScope<'p, P>
where
    P: Scope + HasLock + Into<<Self as Scope>::Parent>,
{
    fn new(guard: &'p mut Guard<'_, P>) -> Self {
        Self {
            parent: guard.inner().into(),
        }
    }
}

struct TryCatch<'p, P>
where
    Self: Scope,
{
    parent: &'p <Self as Scope>::Parent,
}
impl<'p, P> TryCatch<'p, P>
where
    P: Scope + HasLock + Into<<Self as Scope>::Parent>,
{
    fn new(guard: &'p mut Guard<'_, P>) -> Self {
        Self {
            parent: guard.inner().into(),
        }
    }
}

struct ContextScope<'p, P>
where
    Self: Scope,
{
    parent: &'p <Self as Scope>::Parent,
}
impl<'p, P> ContextScope<'p, P>
where
    P: Scope + HasLock + Into<<Self as Scope>::Parent>,
{
    fn new(guard: &'p mut Guard<'_, P>) -> Self {
        Self {
            parent: guard.inner().into(),
        }
    }
}

struct Guard<'s, S>(&'s S)
where
    S: Scope;

impl<'s, S> Guard<'s, S>
where
    S: Scope,
{
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
    type Parent: Scope;
    type Locker;
    type HandleScope;
    type TryCatch;
    type ContextScope;

    fn parent(&self) -> &Self::Parent;
    fn isolate(&self) -> &Isolate {
        self.parent().isolate()
    }
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

impl Scope for Isolate {
    type Parent = Self;
    type Locker = None;
    type HandleScope = None;
    type TryCatch = None;
    type ContextScope = None;
    fn parent(&self) -> &Self::Parent {
        &self
    }
    fn isolate(&self) -> &Isolate {
        self
    }
}

impl<'p, P> Scope for Locker<'p, P>
where
    P: Scope + 'p,
{
    type Parent = P;
    type Locker = Self;
    type HandleScope = P::HandleScope;
    type TryCatch = P::TryCatch;
    type ContextScope = P::ContextScope;
    fn parent(&self) -> &Self::Parent {
        &self.parent
    }
}

impl<'p, P> Scope for Unlocker<'p, P>
where
    P: Scope + HasLock + 'p,
{
    type Parent = P;
    type Locker = Self;
    type HandleScope = P::HandleScope;
    type TryCatch = P::TryCatch;
    type ContextScope = P::ContextScope;
    fn parent(&self) -> &Self::Parent {
        &self.parent
    }
}

impl<'p, P> Scope for HandleScope<'p, P>
where
    P: Scope + 'p,
{
    type Parent = P;
    type Locker = P::Locker;
    type HandleScope = Self;
    type TryCatch = P::TryCatch;
    type ContextScope = P::ContextScope;
    fn parent(&self) -> &Self::Parent {
        &self.parent
    }
}

impl<'p, P> Scope for EscapableHandleScope<'p, P>
where
    P: Scope + 'p,
{
    type Parent = P;
    type Locker = P::Locker;
    type HandleScope = Self;
    type TryCatch = P::TryCatch;
    type ContextScope = P::ContextScope;
    fn parent(&self) -> &Self::Parent {
        &self.parent
    }
}

impl<'p, P> Scope for SealHandleScope<'p, P>
where
    P: Scope + 'p,
{
    type Parent = P;
    type Locker = P::Locker;
    type HandleScope = Self;
    type TryCatch = P::TryCatch;
    type ContextScope = P::ContextScope;
    fn parent(&self) -> &Self::Parent {
        &self.parent
    }
}

impl<'p, P> Scope for TryCatch<'p, P>
where
    P: Scope + 'p,
{
    type Parent = P;
    type Locker = P::Locker;
    type HandleScope = P::HandleScope;
    type TryCatch = Self;
    type ContextScope = P::ContextScope;
    fn parent(&self) -> &Self::Parent {
        &self.parent
    }
}

impl<'p, P> Scope for ContextScope<'p, P>
where
    P: Scope + 'p,
{
    type Parent = P;
    type Locker = P::Locker;
    type HandleScope = P::HandleScope;
    type TryCatch = P::TryCatch;
    type ContextScope = Self;
    fn parent(&self) -> &Self::Parent {
        &self.parent
    }
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
trait HasTryCatch: Scope {
    fn try_catch(&self) -> &Self::TryCatch;
}

impl<'p, P: Scope + 'p> HasLock for Locker<'p, P> {}
impl<'p1, 'p2, P2: Scope> HasHandles for Locker<'p1, Unlocker<'p2, P2>> where
    P2: HasLock + HasHandles
{
}
impl<'p1, 'p2, P2: Scope> HasContext for Locker<'p1, Unlocker<'p2, P2>> where
    P2: HasLock + HasContext
{
}
impl<'p1, 'p2: 'p1, P2: Scope> HasTryCatch for Locker<'p1, Unlocker<'p2, P2>>
where
    P2: HasLock + HasTryCatch + 'p2,
{
    fn try_catch(&self) -> &Self::TryCatch {
        self.parent().parent().try_catch()
    }
}

impl<'p, P: Scope + 'p> HasLock for HandleScope<'p, P> where P: HasLock {}
impl<'p, P: Scope + 'p> HasHandles for HandleScope<'p, P> {}
impl<'p, P: Scope + 'p> HasContext for HandleScope<'p, P> where P: HasContext {}
impl<'p, P: Scope + 'p> HasTryCatch for HandleScope<'p, P>
where
    P: HasTryCatch,
{
    fn try_catch(&self) -> &Self::TryCatch {
        self.parent().try_catch()
    }
}

impl<'p, P: Scope + 'p> HasLock for EscapableHandleScope<'p, P> where P: HasLock {}
impl<'p, P: Scope + 'p> HasHandles for EscapableHandleScope<'p, P> {}
impl<'p, P: Scope + 'p> HasContext for EscapableHandleScope<'p, P> where P: HasContext {}
impl<'p, P: Scope + 'p> HasTryCatch for EscapableHandleScope<'p, P>
where
    P: HasTryCatch,
{
    fn try_catch(&self) -> &Self::TryCatch {
        self.parent().try_catch()
    }
}

impl<'p, P: Scope + 'p> HasLock for SealHandleScope<'p, P> where P: HasLock {}
impl<'p, P: Scope + 'p> HasContext for SealHandleScope<'p, P> where P: HasContext {}
impl<'p, P: Scope + 'p> HasTryCatch for SealHandleScope<'p, P>
where
    P: HasTryCatch,
{
    fn try_catch(&self) -> &Self::TryCatch {
        self.parent().try_catch()
    }
}

impl<'p, P: Scope + 'p> HasLock for ContextScope<'p, P> where P: HasLock {}
impl<'p, P: Scope + 'p> HasHandles for ContextScope<'p, P> where P: HasHandles {}
impl<'p, P: Scope + 'p> HasContext for ContextScope<'p, P> {}
impl<'p, P: Scope + 'p> HasTryCatch for ContextScope<'p, P>
where
    P: HasTryCatch,
{
    fn try_catch(&self) -> &Self::TryCatch {
        self.parent().try_catch()
    }
}

impl<'p, P: Scope + 'p> HasLock for TryCatch<'p, P> where P: HasLock {}
impl<'p, P: Scope + 'p> HasHandles for TryCatch<'p, P> where P: HasHandles {}
impl<'p, P: Scope + 'p> HasContext for TryCatch<'p, P> where P: HasContext {}
impl<'p, P: Scope + 'p> HasTryCatch for TryCatch<'p, P> {
    fn try_catch(&self) -> &Self::TryCatch {
        self
    }
}

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

    let mut g2 = hs2.enter();

    let mut l4 = Local::new(&mut g, "a");
    let mut l3 = Local::new(&mut g2, 1);

    l1.print_mut();
    l2.print_mut();
    l3.print_mut();
    l4.print_mut();

    std::mem::drop(locker);
}
