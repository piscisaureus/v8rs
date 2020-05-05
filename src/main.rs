use derive_deref::*;
use std::marker::PhantomData;

mod param {
    use super::*;
    pub struct In;
    pub struct Nw;
    pub struct InEsc<'e>(PhantomData<&'e ()>);
    pub struct NwEsc<'e>(PhantomData<&'e ()>);

    pub struct No;

    pub trait Param {
        type Raw;
    }
    impl<'a> Param for In {
        type Raw = [usize; 3];
    }
    impl<'a> Param for Nw {
        type Raw = *mut usize;
    }
    impl Param for No {
        type Raw = ();
    }

    pub trait Stable {}
    impl Stable for In {}
    impl<'e> Stable for InEsc<'e> {}
    impl Stable for No {}
}
use param::*;

struct Inner<A>(PhantomData<A>);
impl<A> Drop for Inner<A> {
    fn drop(&mut self) {
        println!("Drop {}", std::any::type_name::<A>())
    }
}
struct Scope<'a, Locals = No, Escape = No, TryCatch = No>(
    Inner<(Locals, Escape, TryCatch)>,
    PhantomData<(&'a ())>,
);

type HandleScope<'a> = Scope<'a, In, No, No>;
type EscapableHandleScope<'a, 'e: 'a> = Scope<'a, In, InEsc<'e>, No>;
type TryCatch<'a, Locals, Escape> = Scope<'a, Locals, Escape, In>;

trait Push<'p, Child> {
    type New;
}

impl<'a, 'p: 'a, Locals, Escape, TryCatch> Push<'p, Scope<'a>>
    for Scope<'p, Locals, Escape, TryCatch>
{
    type New = Self;
}

impl<'a, 'p: 'a, __, Escape: 'a, TryCatch: 'a> Push<'p, HandleScope<'a>>
    for Scope<'p, __, Escape, TryCatch>
{
    type New = Scope<'a, Nw, Escape, TryCatch>;
}

impl<'a, 'e: 'a, 'p: 'a, Escape: 'a, TryCatch: 'a> Push<'p, EscapableHandleScope<'a, 'e>>
    for Scope<'p, In, Escape, TryCatch>
{
    type New = Scope<'a, Nw, NwEsc<'e>, TryCatch>;
}

impl<'a, 'p: 'a, Locals: Stable + 'a, Escape: Stable + 'a, __>
    Push<'p, TryCatch<'a, Locals, Escape>> for Scope<'p, Locals, Escape, __>
{
    type New = Scope<'a, Locals, Escape, Nw>;
}

trait Enter {
    type Entered;
}

impl<'a, TryCatch: Stable> Enter for Scope<'a, Nw, No, TryCatch> {
    type Entered = Scope<'a, In, No, TryCatch>;
}

impl<'a, 'e: 'a, TryCatch: Stable> Enter for Scope<'a, Nw, NwEsc<'e>, TryCatch> {
    type Entered = Scope<'a, In, InEsc<'e>, TryCatch>;
}

impl<'a, Locals, Escape> Enter for Scope<'a, Locals, Escape, Nw> {
    type Entered = Scope<'a, Locals, Escape, In>;
}

impl<'a> HandleScope<'a> {
    fn new<'p: 'a, P: Push<'p, Self>>(parent: &'a mut P) -> P::New {
        fake_it()
    }
}

impl<'a, 'e: 'a> EscapableHandleScope<'a, 'e> {
    fn new<'p: 'a, P: Push<'p, Self>>(parent: &'a mut P) -> P::New {
        fake_it()
    }
}

impl<'a, Locals, Escape> TryCatch<'a, Locals, Escape> {
    fn new<'p: 'a, P: Push<'p, Self>>(parent: &'a mut P) -> P::New {
        fake_it()
    }
}

impl<'a, Locals, Escape, TryCatch> Scope<'a, Locals, Escape, TryCatch>
where
    Self: Enter,
    //Self: LifeTime<'a> + Enter,
    //<Self as Enter>::Entered: LifeTime<'a>,
{
    fn enter(&'a mut self) -> &'a mut <Self as Enter>::Entered {
        fake_it()
    }
}

#[derive(Clone, Copy, Default)]
struct Local<'a> {
    a: usize,
    p: PhantomData<&'a ()>,
}

impl<'a, Escape, TryCatch> Scope<'a, In, Escape, TryCatch> {
    fn new_local(&'_ mut self) -> Local<'a> {
        fake_it()
    }
}

impl<'a> Scope<'a> {
    fn root() -> Self {
        fake_it()
    }
}

fn scoped<'a>(scope: &mut HandleScope<'a>) -> Local<'a> {
    scope.new_local()
}

fn scoped2(scope: &mut Scope) {
    let mut hs = HandleScope::new(scope);
    let hse = hs.enter();
    let l = hse.new_local();
}

fn main() {
    let mut s1 = Scope::root();
    let mut hs = HandleScope::new(&mut s1);
    //let mut tc = TryCatch::new(&mut hs);
    let mut hse = hs.enter();
    let mut es = EscapableHandleScope::new(hse);
    let mut ese = es.enter();
    let mut tc = TryCatch::new(ese);
    let mut tce = tc.enter();

    let mut s2 = Scope::root();
    let x = {
        //let mut tc2 = TryCatch::new(&mut s2);
        //let mut tc2e = tc2.enter();
        //print_type(tc2e);
        let mut h2 = HandleScope::new(&mut s2);
        let mut h2e = h2.enter();
        let l = scoped(h2e);
        //{
        let mut x2 = HandleScope::new(h2e);
        let mut x2e = x2.enter();
        //}
        let l2 = scoped(h2e);
        //let l = h2e.new_local();
        //let mut hs2x = HandleScope::new(h2e);
        //let mut hs2xe = hs2x.enter();
        //let mut hs2 = HandleScope::new(h2e);
        //let mut hs2e = hs2.enter();
        //print_type(hs2xe);
        //l;
    };

    let mut s3 = Scope::root();
    let x = {
        let mut h2 = HandleScope::new(&mut s3);
        let mut h2e = h2.enter();
        let l = h2e.new_local();
        l;
    };
}

fn print_type<T>(_: &T) {
    eprintln!("{}", std::any::type_name::<T>());
}

fn fake_it<T>() -> T {
    unsafe { std::mem::MaybeUninit::<T>::zeroed().assume_init() }
}
