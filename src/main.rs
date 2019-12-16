use std::any::type_name;
use std::default::Default;
use std::fmt::{self, Debug, Formatter};
use std::marker::*;
use std::mem::*;

#[derive(Default, Debug)]
struct IsolateScope(i32);
impl Scope for IsolateScope {}

#[derive(Default, Debug)]
struct Locker(i32);
impl Scope for Locker {}

#[derive(Default, Debug)]
struct Unlocker(i32);
impl Scope for Unlocker {}

#[derive(Default, Debug)]
struct ContextScope(i32);
impl Scope for ContextScope {}

#[derive(Default, Debug)]
struct TryCatch(i32);
impl Scope for TryCatch {}

#[derive(Default, Debug)]
struct HandleScope(i32);
impl Scope for HandleScope {}
impl OpenHandleScope for HandleScope {}

#[derive(Default, Debug)]
struct EscapableHandleScope(i32);
impl Scope for EscapableHandleScope {}
impl OpenHandleScope for EscapableHandleScope {}

#[derive(Default, Debug)]
struct SealHandleScope(i32);
impl Scope for SealHandleScope {}

trait Scope: Debug + Default + Sized {
    fn new<P>(parent: &mut P) -> Frame<Self, P>
    where
        P: ScopeParent,
    {
        Frame::new(parent)
    }

    fn enter<P>(buf: &mut MaybeUninit<Self>, _parent: &mut P)
    where
        P: ScopeParent,
    {
        *buf = MaybeUninit::new(Default::default())
    }
}

trait ScopeParent
where
    Self: Debug,
{
    type Data;
}

trait OpenHandleScope
where
    Self: Scope,
{
}

mod current {
    use super::*;

    pub trait Isolate {}
    pub trait Locking {}
    pub trait Context {}
    pub trait Handles {}
    pub trait TryCatch {}
}

#[derive(Copy, Clone)]
pub struct TypeRef<T>(PhantomData<T>);
pub trait ID: Sized {
    const ID: TypeRef<Self>;
}
impl<T> ID for T {
    const ID: TypeRef<T> = TypeRef(PhantomData);
}

#[allow(dead_code)]
struct Match;
#[allow(dead_code)]
struct Unmatch<Next>(Next);

impl<'p, D, P> Guard<'p, D, P>
where
    D: Scope,
    P: ScopeParent,
{
    fn get<X, M>(&mut self, _: TypeRef<X>) -> &mut <Self as Follows<X, M>>::Guard
    where
        Self: Follows<X, M>,
    {
        Follows::follow(self)
    }
}
trait Follows<D, M> {
    type Guard;
    fn follow(&mut self) -> &mut Self::Guard;
}
impl<'p, D, P> Follows<D, Match> for Guard<'p, D, P>
where
    D: Scope,
    P: ScopeParent,
{
    type Guard = Self;
    fn follow(&mut self) -> &mut Self::Guard {
        self
    }
}
impl<'p, D, P, X, M> Follows<X, Unmatch<M>> for Guard<'p, D, P>
where
    D: Scope,
    P: ScopeParent + Follows<X, M>,
{
    type Guard = <P as Follows<X, M>>::Guard;
    fn follow(&mut self) -> &mut Self::Guard {
        self.parent.follow()
    }
}

#[derive(Default, Debug)]
struct Bottom;
impl ScopeParent for Bottom {
    type Data = ();
}

#[derive(Debug)]
struct Guard<'p, D, P>
where
    P: ScopeParent,
    D: Scope,
{
    parent: &'p mut P,
    data: &'p D,
}

impl<'p, D, P> ScopeParent for Guard<'p, D, P>
where
    P: ScopeParent,
    D: Scope,
{
    type Data = D;
}

impl<'p, D, P> Guard<'p, D, P>
where
    P: ScopeParent,
    D: Scope,
{
    fn new(parent: &'p mut P, data: &'p D) -> Self {
        dump_ret("Guard::new", Self { parent, data })
    }
}

impl<'p, D, P> Drop for Guard<'p, D, P>
where
    P: ScopeParent,
    D: Scope,
{
    fn drop(&mut self) {
        dump("Guard::drop", self);
    }
}

enum Frame<'p, D, P>
where
    D: Scope,
{
    Config(&'p mut P),
    Data(D),
    Uninit(MaybeUninit<D>),
}

fn dump<T: Debug>(m: &'static str, t: &T) {
    println!("{} {:?}", m, &t);
}

fn dump_ret<T: Debug>(m: &'static str, t: T) -> T {
    dump(m, &t);
    t
}

impl<'p, D, P> Frame<'p, D, P>
where
    P: ScopeParent,
    D: Scope,
{
    fn new(parent: &'p mut P) -> Self {
        dump_ret("Frame::new", Self::Config(parent))
    }

    pub fn enter(&'p mut self) -> Guard<'p, D, P> {
        let uninit = || Frame::Uninit(MaybeUninit::uninit());

        let parent = match replace(self, uninit()) {
            Frame::Config(p) => p,
            _ => unreachable!(),
        };

        let uninit_data = match self {
            Frame::Uninit(u) => u,
            _ => unreachable!(),
        };
        let uninit_data_ptr = uninit_data.as_ptr();

        D::enter(uninit_data, parent);

        let data_temp = match replace(self, uninit()) {
            Frame::Uninit(u) => unsafe { u.assume_init() },
            _ => unreachable!(),
        };
        replace(self, Frame::Data(data_temp));

        let data = match self {
            Frame::Data(d) => d,
            _ => unreachable!(),
        };
        let data_ptr = data as *const _;

        assert_eq!(uninit_data_ptr, data_ptr);
        Guard::new(parent, data)
    }
}

impl<'p, D, P> Debug for Frame<'p, D, P>
where
    D: Scope,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(_) => write!(f, "Frame::<{}>::Config", type_name::<D>()),
            Self::Uninit(_) => write!(f, "Frame::<{}>::Uninit", type_name::<D>()),
            Self::Data(data) => write!(f, "Frame<{}>::Data {{ {:?} }}", type_name::<D>(), data),
        }
    }
}

#[derive(Debug)]
struct Local<'p, D, T> {
    scope: &'p D,
    value: T,
}

impl<'p, D, T> Local<'p, D, T>
where
    Self: Debug,
    D: Scope + OpenHandleScope,
{
    pub fn new<P>(parent: &'_ mut Guard<'p, D, P>, value: T) -> Self
    where
        P: ScopeParent,
    {
        dump_ret(
            "Local::new",
            Self {
                scope: parent.data,
                value,
            },
        )
    }

    pub fn print(&self) {
        println!("Local::print {:?}", self);
    }
}

fn main() {
    let mut g0 = Bottom;

    let mut loc = Locker::new(&mut g0);
    let mut gloc = loc.enter();

    {
        let mut hs1 = HandleScope::new(&mut gloc);
        let mut ghs1 = hs1.enter();
        let l1a = Local::new(&mut ghs1, "1a");
        l1a.print();
    }

    let mut hs2 = HandleScope::new(&mut gloc);
    let mut ghs2 = hs2.enter();
    let l2a = Local::new(&mut ghs2, "2a");
    l2a.print();

    println!("=======");

    main1();
}

//#[cfg(off)]
fn main1() {
    let mut g0 = Bottom;

    let mut loc = Locker::new(&mut g0);
    let mut gloc = loc.enter();

    let mut ctx = ContextScope::new(&mut gloc);
    let mut gctx = ctx.enter();

    let mut hs1 = HandleScope::new(&mut gctx);
    let mut ghs1 = hs1.enter();

    let l1a = Local::new(&mut ghs1, "1a");
    let l1b = Local::new(&mut ghs1, "1b");

    let mut hs2 = HandleScope::new(&mut ghs1);
    let mut ghs2 = hs2.enter();

    let v = ghs2.get(Locker::ID);

    let l2a = Local::new(&mut ghs2, "2a");
    let l2b = Local::new(&mut ghs2, "2b");

    let mut hs3 = HandleScope::new(&mut ghs2);
    let mut ghs3 = hs3.enter();

    let l3a = Local::new(&mut ghs3, "3a");

    l3a.print();

    drop(ghs3);

    let mut hs3b = EscapableHandleScope::new(&mut ghs2);
    let mut ghs3b = hs3b.enter();

    let l3b = Local::new(&mut ghs3b, "3b");

    let mut ctx2 = ContextScope::new(&mut ghs3b);
    let mut gctx2 = ctx2.enter();

    let mut ctx3 = ContextScope::new(&mut gctx2);
    let mut gctx3 = ctx3.enter();

    l1a.print();
    l2a.print();
    l3b.print();

    let mut hs4 = HandleScope::new(&mut gctx3);
    let mut hs4 = hs4.enter();
    let l4a = Local::new(&mut hs4, "l4");
    //let mut gctx2 = ctx3.enter();

    l4a.print();

    drop(hs4);
    drop(gctx3);

    let mut ctx3b = ContextScope::new(&mut gctx2);
    let mut gctx3b = ctx3b.enter();

    l1a.print();
    l2a.print();
    l3b.print();
}
