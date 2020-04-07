use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::marker::PhantomPinned;
use std::mem::drop;
use std::ops::*;

struct Dummy<'a>(PhantomData<&'a mut ()>);
impl<'a> Default for Dummy<'a> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
impl<'a> Drop for Dummy<'a> {
    fn drop(&mut self) {}
}

struct ScopeRef<'p, 'a>(&'a mut Scope<'p>);

impl<'p, 'a> Drop for ScopeRef<'p, 'a> {
    fn drop(&mut self) {
        println!("Scope drop");
    }
}

impl<'p, 'a> Deref for ScopeRef<'p, 'a> {
    type Target = Scope<'p>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'p, 'a> DerefMut for ScopeRef<'p, 'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

struct Scope<'a> {
    _parent: PhantomData<&'a ()>,
    _next: Option<Box<UnsafeCell<Scope<'a>>>>,
}

impl<'a> Scope<'a> {
    fn root() -> ScopeRef<'a, 'a> {
        let b = Box::new(Self {
            _parent: PhantomData,
            _next: None,
        });
        let p = Box::into_raw(b);
        ScopeRef(unsafe { &mut *p })
    }

    fn new<'p: 'a>(parent: &'a mut Scope<'p>) -> ScopeRef<'p, 'a> {
        let b = Box::new(UnsafeCell::new(Scope::<'p> {
            _parent: PhantomData,
            _next: None,
        }));
        let p = unsafe { b.get() };
        parent._next = Some(b);
        ScopeRef(unsafe { &mut *p })
    }

    fn make_local<'b>(&'b mut self) -> Local<'a>
    where
        'a: 'b,
    {
        Local {
            _phantom: PhantomData,
        }
    }
}

struct Local<'a> {
    _phantom: PhantomData<&'a ()>,
}

impl<'a> Local<'a> {
    fn new<'b>(scope: &'b mut Scope<'a>) -> Self
    where
        'a: 'b,
    {
        scope.make_local()
    }
}

fn indirect_make_local<'a>(scope: &'_ mut Scope<'a>) -> Local<'a> {
    Local::new(scope)
}

fn use_it<T>(_: &T) {}

fn main() {
    let u;

    let root1 = &mut Scope::root();
    let root2 = &mut Scope::root();
    {
        let x = &mut Scope::new(root1);
        let xxv = x.make_local();
        let yyv = {
            let mut y = Scope::new(root2);
            //std::mem::swap(&mut x, &mut y);
            //let r1 = (y.get_make_local())();
            //let r2 = (y.get_make_local())();
            let r1 = y.make_local();
            let r2 = y.make_local();
            let r3 = Local::new(&mut y);
            let r4 = Local::new(&mut y);
            let r5 = indirect_make_local(&mut y);
            let z1 = {
                let w0 = &mut Scope::new(&mut y);
                let wl0 = Local::new(w0);
                {
                    let w1 = &mut Scope::new(w0);
                    let wl1 = Local::new(w1);
                }
                let w2 = &mut Scope::new(w0);
                let wl0x = Local::new(w0);
                let wl2 = Local::new(w2);
                use_it(&r1);
                use_it(&r2);
                use_it(&r3);
                use_it(&r4);
                wl0
            };
            use_it(&z1);
            let ref mut y2 = Scope::new(&mut y);
            //u = y2;
            //r
            //use_it(&z1);
            //use_it(&r5);
            //std::mem::swap(y2, y);
            let z2 = Local::new(y2);
            let z3 = Scope::new(y2);
            use_it(&r4);
            use_it(&z2);

            u = ();
        };
        let mut y2 = Scope::new(root2);
        //drop(root2);
        //use_it(&xxv);
        //drop(x);
        use_it(&yyv);
        //use_it(u);
    }

    //let mut xb: Scope = Scope::new(&mut x);
    //let mut a = Scope::root();
    //let mut b1 = Scope::new(&mut a);
    //let v1 = Local::new(&mut b1);
    ////std::mem::swap(&mut xb, &mut b1);
    ////let xc = Scope::new(&mut b1);
    //let v2 = Local::new(&mut b1);
    //let mut c = Scope::new(&mut b1);
    ////drop(b1);
    ////drop(b1);
    //drop(v1);
    //println!("Hello, world!");
}
