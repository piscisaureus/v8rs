use std::marker::PhantomData;

struct Isolate(*mut [u8; 0]);

impl Isolate {
    pub fn new() -> Self {
        Self(std::ptr::null_mut())
    }
}

impl std::default::Default for Isolate {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
struct Locker<'p, P> {
    parent: PhantomData<&'p P>,
}
#[derive(Default)]
struct Unlocker<'p, P> {
    parent: PhantomData<&'p P>,
}
#[derive(Default)]
struct HandleScope<'p, P> {
    parent: PhantomData<&'p P>,
}
#[derive(Default)]
struct EscapableHandleScope<'p, P> {
    parent: PhantomData<&'p P>,
}
#[derive(Default)]
struct SealHandleScope<'p, P> {
    parent: PhantomData<&'p P>,
}
#[derive(Default)]
struct TryCatch<'p, P> {
    parent: PhantomData<&'p P>,
}
#[derive(Default)]
struct ContextScope<'p, P> {
    parent: PhantomData<&'p P>,
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

    fn enter(&mut self) -> Guard<Self> {
        Guard::new(self)
    }
}

impl Scope for Isolate {
    type Parent = Self;
    type Locker = None;
    type HandleScope = None;
    type TryCatch = None;
    type ContextScope = None;
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
}

trait LockGuard<'s> {}
impl<'s, S> LockGuard<'s> for Guard<'s, S> where S: Scope + HasLock + 's {}

trait HandleScopeGuard<'s> {}
impl<'s, S> HandleScopeGuard<'s> for Guard<'s, S> where S: Scope + HasLock + HasHandles + 's {}

//impl<'s, S> HasContext for Guard<'s, S> where S: Scope + HasLock + HasContext {}
//impl<'s, S> HasTryCatch for Guard<'s, S> where S: Scope + HasLock + HasHandles + HasTryCatch {}

trait HasLock {}
trait HasHandles {}
trait HasContext {}
trait HasTryCatch {}

impl<'p, P> HasLock for Locker<'p, P> {}
impl<'p1, 'p2, P2> HasHandles for Locker<'p1, Unlocker<'p2, P2>> where P2: HasHandles {}
impl<'p1, 'p2, P2> HasContext for Locker<'p1, Unlocker<'p2, P2>> where P2: HasContext {}
impl<'p1, 'p2, P2> HasTryCatch for Locker<'p1, Unlocker<'p2, P2>> where P2: HasTryCatch {}

impl<'p, P> HasLock for HandleScope<'p, P> where P: HasLock {}
impl<'p, P> HasHandles for HandleScope<'p, P> {}
impl<'p, P> HasContext for HandleScope<'p, P> where P: HasContext {}
impl<'p, P> HasTryCatch for HandleScope<'p, P> where P: HasTryCatch {}

impl<'p, P> HasLock for EscapableHandleScope<'p, P> where P: HasLock {}
impl<'p, P> HasHandles for EscapableHandleScope<'p, P> {}
impl<'p, P> HasContext for EscapableHandleScope<'p, P> where P: HasContext {}
impl<'p, P> HasTryCatch for EscapableHandleScope<'p, P> where P: HasTryCatch {}

impl<'p, P> HasLock for SealHandleScope<'p, P> where P: HasLock {}
impl<'p, P> HasContext for SealHandleScope<'p, P> where P: HasContext {}
impl<'p, P> HasTryCatch for SealHandleScope<'p, P> where P: HasTryCatch {}

impl<'p, P> HasLock for ContextScope<'p, P> where P: HasLock {}
impl<'p, P> HasHandles for ContextScope<'p, P> where P: HasHandles {}
impl<'p, P> HasContext for ContextScope<'p, P> {}
impl<'p, P> HasTryCatch for ContextScope<'p, P> where P: HasTryCatch {}

impl<'p, P> HasLock for TryCatch<'p, P> where P: HasLock {}
impl<'p, P> HasHandles for TryCatch<'p, P> where P: HasHandles {}
impl<'p, P> HasContext for TryCatch<'p, P> where P: HasContext {}
impl<'p, P> HasTryCatch for TryCatch<'p, P> {}

trait NewScope<'p, P>
where
    Self: Scope<Parent = P> + Default,
    P: Scope + 'p,
{
    fn new(parent: &'_ mut Guard<'p, P>) -> Self {
        Default::default()
    }
}

impl<'p, S, P> NewScope<'p, P> for S
where
    S: Scope<Parent = P> + Default,
    P: Scope + 'p,
{
}

fn main() {
    println!("Hello, world!");

    let mut isolate = Isolate::new();
    let mut g = isolate.enter();

    let mut locker = Locker::new(&mut g);
    let mut g = locker.enter();

    let mut hs1 = HandleScope::new(&mut g);
    let mut g1 = hs1.enter();

    let mut l1 = Local::new(&mut g1, 1);
    let mut l2 = Local::new(&mut g1, "a");

    let mut hs2 = HandleScope::new(&mut g1);
    let mut g2 = hs2.enter();

    let mut l4 = Local::new(&mut g1, "a");
    let mut l3 = Local::new(&mut g2, 1);

    l1.print_mut();
    l2.print_mut();
    l3.print_mut();
    l4.print_mut();

    std::mem::drop(locker);
}
