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
        P: Debug,
    {
        Frame::new(parent)
    }

    fn enter<P>(buf: &mut MaybeUninit<Self>, _parent: &mut P) {
        *buf = MaybeUninit::new(Default::default())
    }
}

trait OpenHandleScope
where
    Self: Scope,
{
}

#[derive(Default, Debug)]
struct Bottom;

#[derive(Debug)]
struct Guard<'p, D, P>
where
    D: Scope,
    P: Debug,
{
    parent: &'p mut P,
    data: &'p D,
}

impl<'p, D, P> Guard<'p, D, P>
where
    D: Scope,
    P: Debug,
{
    fn new(data: &'p D, parent: &'p mut P) -> Self {
        dump_ret("Guard::new", Self { parent, data })
    }
}

impl<'p, D, P> Drop for Guard<'p, D, P>
where
    D: Scope,
    P: Debug,
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
    D: Scope,
    P: Debug,
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
        Guard::new(data, parent)
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
        P: Debug,
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

#[derive(Debug, Default)]
struct Nothing;
impl Scope for Nothing {}

#[repr(C)]
#[derive(Debug, Default)]
struct CxxIsolate([u8; 0]);
impl Scope for CxxIsolate {}
#[repr(C)]
#[derive(Debug, Default)]
struct CxxLocker([usize; 3]);
impl Scope for CxxLocker {}
#[repr(C)]
#[derive(Debug, Default)]
struct CxxHandleScope([usize; 3]);
impl Scope for CxxHandleScope {}
#[repr(C)]
#[derive(Debug, Default)]
struct CxxEscapableHandleScope([usize; 4]);
impl Scope for CxxEscapableHandleScope {}
#[repr(C)]
#[derive(Debug, Default)]
struct CxxContextScope([usize; 3]);
impl Scope for CxxContextScope {}
#[repr(C)]
#[derive(Debug, Default)]
struct CxxTryCatch([usize; 6]);
impl Scope for CxxTryCatch {}

trait Scope2<'p> {
    type Data;
    type Parent;
    type Guard;
    fn enter(&'p mut self) -> Self::Guard;
}

trait NewHandle<T> {
    type Handle;
}

impl<'p, D, P> Scope2<'p> for Frame<'p, D, P>
where
    D: Scope + 'p,
    P: Debug,
{
    type Parent = P;
    type Data = D;
    type Guard = Guard<'p, D, P>;

    fn enter(&'p mut self) -> Self::Guard {
        self.enter()
    }
}

struct IsolateScope2<'p>(&'p CxxIsolate);

impl<'p> IsolateScope2<'p> {
    pub fn new(isolate: &'p CxxIsolate) -> Self {
        Self(isolate)
    }
}

impl<'p> Scope2<'p> for IsolateScope2<'p> {
    type Parent = &'p CxxIsolate;
    type Data = Nothing;
    type Guard = Guard<'p, Self::Data, Self::Parent>;

    fn enter(&'p mut self) -> Self::Guard {
        Guard::new(&Nothing, &mut self.0)
    }
}

type HandleScope2<'p, P> = Frame<'p, CxxHandleScope, P>;

impl<'p, P, T> NewHandle<T> for HandleScope2<'p, P>
where
    Self: Scope2<'p>,
{
    type Handle = Local<'p, <Self as Scope2<'p>>::Data, T>;
}

/*
struct ScopeBuf<S> where S: ScopeImpl {
    parent: S::Parent
    data: S::Data
}

impl<S> ScopeBuf<S> where S: ScopeImpl {
    fn new(parent: S::Parent, inner: ScopeBufInner<S>) -> Self {
        Self{ parent, inner }
    }
}





trait ScopeImpl {
    type Parent;
    type CxxData;

    fn isolate(&self) -> &CxxIsolate;
    fn isolate_mut(&mut self) -> &mut CxxIsolate;
}

trait IsolateScope<'p> = Frame<'p, (), &'mut CxxIsolate> {
    type Parent = &'mut CxxIsolate;

}

trait ScopeImpl: Debug + Default + Sized {
    fn new<P>(parent: &mut P) -> Frame<Self, P>
    where
        {
        Frame::new(parent)
    }

    fn enter<P>(buf: &mut MaybeUninit<Self>, _parent: &mut P)
    where
        {
        *buf = MaybeUninit::new(Default::default())
    }
}
*/
