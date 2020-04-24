#[macro_use]
use derive_deref::*;
use std::cell::Cell;
use std::cell::RefCell;
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::ops::*;
use std::ptr::null_mut;
use std::ptr::NonNull;

trait ScopeTrait {
    fn close(&mut self) {}
}

mod a {
    use std::any::Any;
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::cell::RefMut;
    use std::cell::UnsafeCell;
    use std::collections::VecDeque;
    use std::convert::TryFrom;
    use std::marker::PhantomData;
    use std::mem::replace;
    use std::mem::transmute;
    use std::mem::*;
    use std::ops::*;
    use std::ptr;
    use std::ptr::*;
    use std::rc::Rc;
    #[macro_use]
    use derive_deref::*;

    #[derive(Clone, Copy)]
    pub struct ScopeTop {
        handle_scope: *mut HandleScopeData,
        escape_slot: *mut EscapeSlotData,
        try_catch: *mut TryCatchData,
    }

    impl Default for ScopeTop {
        fn default() -> Self {
            unsafe { MaybeUninit::zeroed().assume_init() }
        }
    }

    #[derive(Default)]
    pub struct ScopeManager {
        depth: Cell<usize>,
        inner: RefCell<ScopeManagerInner>,
    }

    impl ScopeManager {
        pub fn new() -> Rc<Self> {
            Rc::new(Default::default())
        }

        #[allow(clippy::mut_from_ref)]
        fn get(&self, depth: usize) -> &RefCell<ScopeManagerInner> {
            assert_eq!(depth, self.depth.get());
            &self.inner
        }

        fn new_root(&self) -> usize {
            let depth = self.depth.get() + 1;
            self.depth.set(depth);
            depth
        }

        fn shadow(&self, depth: usize) -> usize {
            assert_eq!(depth, self.depth.get());
            depth += 1;
            self.depth.set(depth);
            depth
        }

        fn unshadow(&self, depth: usize) {
            assert_eq!(depth, self.depth.get());
            self.depth.set(depth - 1);
        }
    }

    pub struct ScopeManagerInner {
        top: ScopeTop,
        stack: Vec<u8>,
    }

    impl Default for ScopeManagerInner {
        fn default() -> Self {
            Self {
                top: Default::default(),
                stack: Vec::with_capacity(Self::SCOPE_STACK_SIZE),
            }
        }
    }

    impl ScopeManagerInner {
        const SCOPE_STACK_SIZE: usize = 4096 - size_of::<usize>();

        pub fn push<D: ScopeStackItemTrait>(&mut self, data: D) {
            let mut scope_stack = &mut self.stack;
            let frame_byte_length = size_of::<ScopeStackItemData<D>>();
            let stack_byte_offset = scope_stack.len();
            let new_stack_byte_length = stack_byte_offset + frame_byte_length;
            assert!(new_stack_byte_length <= scope_stack.capacity());
            unsafe { scope_stack.set_len(new_stack_byte_length) };

            let frame = unsafe {
                let frame_ptr =
                    scope_stack[stack_byte_offset] as *mut u8 as *mut ScopeStackItemData<D>;
                let data_ptr: *mut D = &mut (*frame_ptr).data;
                let meta_ptr: *mut _ = &mut (*frame_ptr).meta;

                D::construct(data_ptr);

                let meta = ScopeStackItemMeta {
                    previous_top: replace(D::get_top_slot(&mut self.top), data_ptr) as *mut (),
                    cleanup_fn: Self::cleanup_frame::<D>,
                    frame_byte_length,
                };
                ptr::write(meta_ptr, meta);

                &mut *frame_ptr
            };
            assert_eq!(size_of_val(frame), frame.meta.frame_byte_length);
        }

        pub fn pop(&mut self) {
            let mut scope_stack = &mut self.stack;
            let meta_byte_length = size_of::<ScopeStackItemMeta>();
            let meta_byte_offset = scope_stack.len() - meta_byte_length;
            let meta_ptr = scope_stack[meta_byte_offset] as *mut u8 as *mut ScopeStackItemMeta;
            let meta = unsafe { ptr::read(meta_ptr) };
            let ScopeStackItemMeta {
                cleanup_fn,
                frame_byte_length,
                ..
            } = meta;

            let frame_byte_offset = scope_stack.len() - frame_byte_length;
            let frame_ptr = scope_stack[frame_byte_offset] as *mut u8 as *mut ();
            cleanup_fn(frame_ptr, &mut self.top);

            unsafe { scope_stack.set_len(frame_byte_offset) };
        }

        #[allow(dead_code)] // False alarm.
        fn cleanup_frame<D: ScopeStackItemTrait>(frame_ptr: *mut (), top: &mut ScopeTop) {
            let frame_ptr = frame_ptr as *mut ScopeStackItemData<D>;
            let frame = unsafe { &mut *frame_ptr };
            replace(D::get_top_slot(top), frame.meta.previous_top as *mut D);
            unsafe { drop_in_place(&mut frame.data) }
        }
    }

    pub trait ScopeStackItemTrait {
        fn construct(buf: *mut Self) {}
        fn get_top_slot(top: &mut ScopeTop) -> &mut *mut Self;
    }

    struct ScopeStackItemData<D> {
        data: D,
        meta: ScopeStackItemMeta,
    }

    struct ScopeStackItemMeta {
        previous_top: *mut (),
        cleanup_fn: fn(*mut (), &mut ScopeTop) -> (),
        frame_byte_length: usize,
    }

    struct Scope<'l, 'e, 'x> {
        mgr: Rc<ScopeManager>,
        depth: usize,
        _lifetimes: PhantomData<(&'l (), &'e mut (), &'x mut ())>,
    }

    impl<'l, 'e, 'x> Scope<'l, 'e, 'x> {
        pub fn root(mgr: &Rc<ScopeManager>) -> Self {
            let mgr = mgr.clone();
            let depth = mgr.new_root();
            Self {
                mgr,
                depth,
                _lifetimes: PhantomData,
            }
        }
    }

    #[derive(Deref, DerefMut)]
    struct HandleScope<'l>(EscapableHandleScope<'l, 'l>);

    #[derive(Deref, DerefMut)]
    struct EscapableHandleScope<'l, 'e: 'l>(Scope<'l, 'e, 'e>);

    impl<'l, 'e: 'l> EscapableHandleScope<'l, 'e> {
        pub fn new<'pl: 'l + 'e, 'pe, 'px>(
            parent: &mut Scope<'pl, 'pe, 'px>,
        ) -> Scope<'l, 'e, 'px> {
            self
        }
    }

    #[derive(Default)]
    struct HandleScopeData([usize; 3]);
    #[derive(Default)]
    struct EscapeSlotData([usize; 1]);
    #[derive(Default)]
    struct TryCatchData([usize; 7]);

    /*
    #[derive(Copy, Clone)]
    struct ScopeRef<'a, 'l: 'a, 'r: 'a, 'e: 'a> {
        top: &'a UnsafeCell<ScopeTop<'l, 'r, 'e>>,
        depth: usize,
    }

    impl<'a, 'l, 'r, 'e> ScopeRef<'a, 'l, 'r, 'e> {
        fn get_top(&mut self) -> &mut ScopeTop<'l, 'r, 'e> {
            assert_eq!(unsafe { &*(self.top.get()) }.depth, self.depth);
            unsafe { &mut *self.top.get() }
        }
    }

    pub trait AsScope<'a, 'l: 'a, 'r: 'a, 'e: 'a> {
        fn get_ref(&self) -> ScopeRef<'a, 'l, 'r, 'e>;
    }

    #[repr(transparent)]
    struct TryCatch<'l, 'r, 'e>(ScopeRef<'e, 'l, 'r, 'e>);

    impl<'l, 'r, 'e> TryCatch<'l, 'r, 'e> {
        fn new<'p>(parent: &'e mut impl AsScope<'e, 'l, 'r, 'p>) -> Self
        where
            'l: 'e,
            'r: 'e,
            'p: 'e,
        {
            let mut top_ref = parent.get_ref();
            let depth = {
                let mut top: &mut ScopeTop<'l, 'r, 'p> = top_ref.get_top();
                let idx = top.stack.len();
                top.stack
                    .push(RefCell::new(ScopeData::TryCatch(TryCatchData::default())));
                let top: &'e mut ScopeTop<'l, 'r, 'e> = unsafe { transmute(top) };
                {
                    let ref_mut = top.stack.get(idx).unwrap().borrow_mut();
                    top.try_catch = RefMut::map(ref_mut, |d| {
                        if let ScopeData::TryCatch(d) = d {
                            d
                        } else {
                            panic!()
                        }
                    });
                }
                top.depth += 1;
                top.depth
            };
            Self(ScopeRef {
                top: unsafe { transmute(top_ref.top) },
                depth,
            })
        }
    }

    impl<'l, 'r, 'e> AsScope<'e, 'l, 'r, 'e> for TryCatch<'l, 'r, 'e> {
        fn get_ref(&self) -> ScopeRef<'e, 'l, 'r, 'e> {
            self.0
        }
    }

    #[repr(transparent)]
    struct HandleScope<'l>(ScopeRef<'l, 'l, 'l, 'l>);

    impl<'l: 'e, 'r: 'e, 'e> Deref for TryCatch<'l, 'r, 'e> {
        type Target = HandleScope<'e>;
        fn deref(&self) -> &Self::Target {
            //unsafe { std::mem::transmute(self) }
            let s: Self::Target = HandleScope::<'e>(self.0);
            &s
        }
    }
    */

    // impl<'loc, 'res, 'exc> Deref for Scope<'loc, 'res, 'exc> {
    //     type Target = TryCatch<'loc, 'res, 'exc>;
    //     fn deref(&self) -> &Self::Target {
    //         Cell::update(assert_eq!(self.depth, Cell::get(&self.top).depth);
    //     }
    // }
    // impl<'loc, 'res, 'exc> DerefMut for Scope<'loc, 'res, 'exc> {
    //     fn deref_mut(&mut self) -> &mut Self::Target {
    //         &mut *self.top.borrow_mut()
    //     }
    // }
}

#[derive(Debug)]
struct ScopeRef<'s, S>(&'s mut S)
where
    S: ScopeTrait;

impl<'s, S> Drop for ScopeRef<'s, S>
where
    S: ScopeTrait,
{
    fn drop(&mut self) {
        println!("Scope drop.");
        self.0.close()
        //println!(" depth={}, next={:?}", self.depth, self._next);
        //let mut temp = std::ptr::NonNull::<&'a mut Scope<'p>>::dangling();
        //let mut cell = std::mem::replace(&mut self.0, unsafe { std::mem::transmute(temp) });
        //cell._parent.map(|mut v| unsafe { v.as_mut() }._next.take());
    }
}

impl<'s, S> Deref for ScopeRef<'s, S>
where
    S: ScopeTrait,
{
    type Target = S;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'s, S> DerefMut for ScopeRef<'s, S>
where
    S: ScopeTrait,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
struct Scope<'a> {
    _parent: Option<NonNull<Scope<'a>>>,
    _next: Option<Box<UnsafeCell<Scope<'a>>>>,
    depth: usize,
}

impl<'a> Scope<'a> {
    fn root() -> ScopeRef<'a, Self> {
        let b = Box::new(Self {
            _parent: None,
            _next: None,
            depth: 0,
        });
        let p = Box::into_raw(b);
        ScopeRef(unsafe { &mut *p })
    }

    //fn new<'p: 'a>(parent: &'a mut Scope<'p>) -> ScopeRef<'a, Scope<'p>> {
    //    let b = Box::new(UnsafeCell::new(Scope::<'p> {
    //        _parent: NonNull::new(parent),
    //        _next: None,
    //        depth: parent.depth + 1,
    //    }));
    //    let p = b.get();
    //    parent._next = Some(b);
    //    ScopeRef(unsafe { &mut *p })
    //}

    fn new<'p: 'a>(parent: &'a mut Scope<'p>) -> ScopeRef<'a, Scope<'a>> {
        let b = Box::new(UnsafeCell::new(Scope::<'p> {
            _parent: NonNull::new(parent),
            _next: None,
            depth: parent.depth + 1,
        }));
        let p = b.get();
        parent._next = Some(b);
        ScopeRef(unsafe { std::mem::transmute(p) })
    }

    fn make_local<'b, T>(&'b mut self) -> Local<'a, T>
    where
        'a: 'b,
    {
        Local {
            _phantom: PhantomData,
            _ptr: null_mut(),
        }
    }
}

impl<'a> ScopeTrait for Scope<'a> {}

struct Value(*mut ());

#[derive(Copy, Clone)]
struct Local<'a, T> {
    _phantom: PhantomData<&'a T>,
    _ptr: *mut T,
}

struct Global<T> {
    _phantom: PhantomData<T>,
    _ptr: *mut T,
}

impl<T> Global<T> {
    fn new() -> Self {
        Self {
            _phantom: PhantomData,
            _ptr: null_mut(),
        }
    }
}

impl<T> Deref for Global<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self._ptr }
    }
}

impl<'a, T> Local<'a, T> {
    fn new<'b>(scope: &'b mut Scope<'a>) -> Self
    where
        'a: 'b,
    {
        scope.make_local::<T>()
    }
}

impl<'a, T> Deref for Local<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self._ptr }
    }
}

fn indirect_make_local<'a, T>(scope: &'_ mut Scope<'a>) -> Local<'a, T> {
    Local::new(scope)
}

fn use_it<T>(_: &T) {}

fn use_local<T>(_: &T) {}

struct Stuff<'a>(&'a Value, &'a Value, &'a Value);

fn main() {
    let root1 = &mut Scope::root();
    let root2 = &mut Scope::root();
    {
        let x = &mut Scope::new(root1);
        let _xxv = x.make_local::<Value>();
        let yyv = {
            let mut y = Scope::new(x);
            //std::mem::swap(&mut x, &mut y);
            //let r1 = Local::<Value>::new(x);
            //let r2 = (y.get_make_local())();
            let r1 = y.make_local::<Value>();
            let r2 = y.make_local::<Value>();
            let r3 = Local::<Value>::new(&mut y);
            use_local(&r3);
            let r4 = Local::<Value>::new(&mut y);
            use_local(&r3);
            let g = Some(Global::<Value>::new());
            let stuff = Stuff(&r1, &r2, g.as_ref().unwrap());
            //g.replace(Global::new());
            use_local(&r1);
            use_local(g.as_ref().unwrap());
            use_it(&stuff);
            let _r5 = indirect_make_local::<Value>(&mut y);
            let z1 = {
                let w0 = &mut Scope::new(&mut y);
                let wl0 = Local::<Value>::new(w0);
                {
                    let w1 = &mut Scope::new(w0);
                    let _wl1 = Local::<Value>::new(w1);
                }
                let w2 = &mut Scope::new(w0);
                //let wl0x = Local::<Value>::new(w0);
                let _wl2 = Local::<Value>::new(w2);
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
            let z2 = Local::<Value>::new(y2);
            let _z3 = Scope::new(y2);
            use_it(&r4);
            use_it(&z2);
        };
        let _y2 = Scope::new(root2);
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
