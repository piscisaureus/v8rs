use std::any::type_name;
use std::default::Default;
use std::fmt::{self, Debug, Formatter};
use std::marker::*;
use std::mem::*;

#[derive(Default, Debug)]
struct HS(i32);
impl Scope for HS {}
impl HandleScope for HS {}
#[derive(Default, Debug)]
struct HSe(i32);
impl Scope for HSe {}
impl HandleScope for HSe {}
#[derive(Default, Debug)]
struct HSs(i32);
impl Scope for HSs {}
#[derive(Default, Debug)]
struct Loc(i32);
impl Scope for Loc {}
#[derive(Default, Debug)]
struct Ctx(i32);
impl Scope for Ctx {}

trait Scope: Debug + Default + Sized {
    fn new<P>(parent: &mut P) -> Frame<P, Self>
    where
        P: Parent,
    {
        Frame::new(parent)
    }

    fn enter<P>(buf: &mut MaybeUninit<Self>, _parent: &mut P)
    where
        P: Parent,
    {
        *buf = MaybeUninit::new(Default::default())
    }
}

trait Parent
where
    Self: Debug,
{
}

#[derive(Default, Debug)]
struct Bottom;
impl Parent for Bottom {}

#[derive(Debug)]
struct Guard<'p, P, D>
where
    P: Parent,
    D: Scope,
{
    parent: &'p mut P,
    data: &'p D,
}

impl<'p, P, D> Parent for Guard<'p, P, D>
where
    P: Parent,
    D: Scope,
{
}

impl<'p, P, D> Guard<'p, P, D>
where
    P: Parent,
    D: Scope,
{
    fn new(parent: &'p mut P, data: &'p D) -> Self {
        dump_ret("Guard::new", Self { parent, data })
    }
}

impl<'p, P, D> Drop for Guard<'p, P, D>
where
    P: Parent,
    D: Scope,
{
    fn drop(&mut self) {
        dump("Guard::drop", self);
    }
}

enum Frame<'p, P, D>
where
    D: Scope,
{
    Config(&'p mut P),
    UninitData(MaybeUninit<D>),
    Data(D),
}

fn dump<T: Debug>(m: &'static str, t: &T) {
    println!("{} {:?}", m, &t);
}

fn dump_ret<T: Debug>(m: &'static str, t: T) -> T {
    dump(m, &t);
    t
}

impl<'p, P, D> Frame<'p, P, D>
where
    P: Parent,
    D: Scope,
{
    fn new(parent: &'p mut P) -> Self {
        dump_ret("Frame::new", Self::Config(parent))
    }

    pub fn enter(&'p mut self) -> Guard<'p, P, D> {
        let uninit = || Frame::UninitData(MaybeUninit::uninit());

        let parent = match replace(self, uninit()) {
            Frame::Config(p) => p,
            _ => unreachable!(),
        };

        let uninit_data = match self {
            Frame::UninitData(u) => u,
            _ => unreachable!(),
        };
        let uninit_data_ptr = uninit_data.as_ptr();

        D::enter(uninit_data, parent);

        let data_temp = match replace(self, uninit()) {
            Frame::UninitData(u) => unsafe { u.assume_init() },
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

impl<'p, P, D> Debug for Frame<'p, P, D>
where
    D: Scope,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(_) => write!(f, "Frame::<{}>::Config", type_name::<D>()),
            Self::UninitData(_) => write!(f, "Frame::<{}>::UninitData", type_name::<D>()),
            Self::Data(data) => write!(f, "Frame<{}>::Data {{ {:?} }}", type_name::<D>(), data),
        }
    }
}

trait HandleScope {}

#[derive(Debug)]
struct Local<'p, D, T> {
    scope: &'p D,
    value: T,
}

impl<'p, D, T> Local<'p, D, T>
where
    Self: Debug,
    D: Scope + HandleScope,
{
    pub fn new<P>(parent: &'_ mut Guard<'p, P, D>, value: T) -> Self
    where
        P: Parent,
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

    let mut loc = Loc::new(&mut g0);
    let mut gloc = loc.enter();

    {
        let mut hs1 = HS::new(&mut gloc);
        let mut ghs1 = hs1.enter();
        let l1a = Local::new(&mut ghs1, "1a");
        l1a.print();
    }

    let mut hs2 = HS::new(&mut gloc);
    let mut ghs2 = hs2.enter();
    let l2a = Local::new(&mut ghs2, "2a");
    l2a.print();

    println!("=======");

    main1();
}

//#[cfg(off)]
fn main1() {
    let mut g0 = Bottom;

    let mut loc = Loc::new(&mut g0);
    let mut gloc = loc.enter();
    let mut ctx = Ctx::new(&mut gloc);
    let mut gctx = ctx.enter();

    let mut hs1 = HS::new(&mut gctx);
    let mut ghs1 = hs1.enter();

    let l1a = Local::new(&mut ghs1, "1a");
    let l1b = Local::new(&mut ghs1, "1b");

    let mut hs2 = HS::new(&mut ghs1);
    let mut ghs2 = hs2.enter();

    let l2a = Local::new(&mut ghs2, "2a");
    let l2b = Local::new(&mut ghs2, "2b");

    let mut hs3 = HS::new(&mut ghs2);
    let mut ghs3 = hs3.enter();

    let l3a = Local::new(&mut ghs3, "3a");

    l3a.print();

    drop(ghs3);

    let mut hs3b = HSe::new(&mut ghs2);
    let mut ghs3b = hs3b.enter();

    let l3b = Local::new(&mut ghs3b, "3b");

    let mut ctx2 = Ctx::new(&mut ghs3b);
    let mut gctx2 = ctx2.enter();

    let mut ctx3 = Ctx::new(&mut gctx2);
    let mut gctx3 = ctx3.enter();

    l1a.print();
    l2a.print();
    l3b.print();

    let mut hs4 = HS::new(&mut gctx3);
    let mut hs4 = hs4.enter();
    let l4a = Local::new(&mut hs4, "l4");
    //let mut gctx2 = ctx3.enter();

    l4a.print();

    drop(hs4);
    drop(gctx3);

    let mut ctx3b = Ctx::new(&mut gctx2);
    let mut gctx3b = ctx3b.enter();

    l1a.print();
    l2a.print();
    l3b.print();
}
