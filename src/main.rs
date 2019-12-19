use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ops::DerefMut;

#[derive(Default)]
struct HandleScopeData {
    val: usize,
}

impl HandleScopeData {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Drop for HandleScopeData {
    fn drop(&mut self) {}
}

type ChildData = UnsafeCell<[usize; 10]>;

struct Root(ChildData);

struct Scope<'p, D, P> {
    parent: &'p mut P,
    child_data: ChildData,
    data: &'p D,
}

pub trait GetChildData<'p>
where
    Self: 'p,
{
    fn child_data_raw(&'_ self) -> &'p ChildData;
    fn child_data<D>(&'_ self) -> &'p D {
        unsafe { &*(self.child_data_raw() as *const _ as *const D) }
    }
}

impl<'p> GetChildData<'p> for Root {
    fn child_data_raw(&'_ self) -> &'p ChildData {
        unsafe { std::mem::transmute(&self.0) }
    }
}

impl<'a, 'p, D, P> GetChildData<'a> for Scope<'p, D, P>
where
    Self: 'a,
{
    fn child_data_raw(&'_ self) -> &'a ChildData {
        unsafe { std::mem::transmute(&self.child_data) }
    }
}

impl<'p, D, P> Scope<'p, D, P>
where
    P: GetChildData<'p> + 'p,
{
    pub fn new(parent: &'p mut P, data: D) -> Self {
        Self {
            data: parent.child_data(),
            parent,
            child_data: Default::default(),
        }
    }
}

impl<'p, D, P> Deref for Scope<'p, D, P>
where
    P: GetChildData<'p> + 'p,
{
    type Target = &'p D;
    fn deref(&'_ self) -> &'_ &'p D {
        &self.data
    }
}

impl<'p, D, P> Drop for Scope<'p, D, P> {
    fn drop(&mut self) {}
}

fn use_it<T>(v: &T) {}

fn use_deref(v: &HandleScopeData) {}

fn main() {
    let mut s0 = Root(Default::default());
    let mut s1 = Scope::new(&mut s0, HandleScopeData::new());
    let d1: &HandleScopeData = *s1;
    use_deref(&s1);
    let mut s2a = Scope::new(&mut s1, HandleScopeData::new());
    let d2a = ();
    //let d2a: &HandleScopeData = &s2a;
    //let d2a2: &HandleScopeData = &s2a;
    //let d2a3: &HandleScopeData = &s2a;
    drop(s2a);
    let mut s1b = Scope::new(&mut s1, HandleScopeData::new());
    let d2b = &*s1b;

    //let mut s2b = Scope::new(&mut s1, "bla");
    //let d2b = s2b.data();

    //use_it(d1);
    //drop(s2a);
    use_it(&d1);
    use_it(&d2a);
    use_it(&d2b);
}
