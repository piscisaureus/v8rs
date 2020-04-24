use std::cell::Cell;
use std::cell::RefCell;
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::replace;
use std::mem::size_of;
use std::mem::size_of_val;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr;
use std::ptr::drop_in_place;
use std::ptr::null_mut;
use std::rc::Rc;

#[derive(Clone, Copy)]
pub struct ScopeTop {
    handle_scope: *mut HandleScopeData,
    escape_slot: *mut EscapeSlotData,
    _try_catch: *mut TryCatchData,
}

impl Default for ScopeTop {
    fn default() -> Self {
        unsafe { MaybeUninit::zeroed().assume_init() }
    }
}

#[derive(Default)]
pub struct ScopeManager {
    cookie: Cell<u32>,
    inner: RefCell<ScopeManagerInner>,
}

impl Drop for ScopeManager {
    fn drop(&mut self) {
        assert_eq!(self.cookie.get(), 0);
    }
}

impl ScopeManager {
    pub fn new() -> Rc<Self> {
        Rc::new(Default::default())
    }

    #[allow(clippy::mut_from_ref)]
    fn get(&self, cookie: u32) -> &RefCell<ScopeManagerInner> {
        assert_eq!(cookie, self.cookie.get());
        &self.inner
    }

    fn new_root(&self) -> u32 {
        let cookie = self.cookie.get() + 1;
        self.cookie.set(cookie);
        cookie
    }

    fn shadow(&self, mut cookie: u32) -> u32 {
        assert_eq!(cookie, self.cookie.get());
        cookie += 1;
        self.cookie.set(cookie);
        cookie
    }

    fn unshadow(&self, cookie: u32) {
        assert_eq!(cookie, self.cookie.get());
        self.cookie.set(cookie - 1);
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

impl Drop for ScopeManagerInner {
    fn drop(&mut self) {
        assert_eq!(self.stack.len(), 0);
    }
}

impl ScopeManagerInner {
    const SCOPE_STACK_SIZE: usize = 4096 - size_of::<usize>();

    pub fn push<D: ScopeStackItemData>(&mut self) -> *mut D {
        let scope_stack = &mut self.stack;
        let frame_byte_length = size_of::<ScopeStackItemFrame<D>>();
        let stack_byte_offset = scope_stack.len();
        let new_stack_byte_length = stack_byte_offset + frame_byte_length;
        assert!(new_stack_byte_length <= scope_stack.capacity());
        unsafe { scope_stack.set_len(new_stack_byte_length) };

        let frame = unsafe {
            let frame_ptr = scope_stack.get_mut(stack_byte_offset).unwrap() as *mut u8
                as *mut ScopeStackItemFrame<D>;
            let data_cell: &mut UnsafeCell<D> = &mut (*frame_ptr).data;
            let data_ptr = data_cell.get();
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
        frame.data.get()
    }

    pub fn pop(&mut self) {
        let scope_stack = &mut self.stack;
        let meta_byte_length = size_of::<ScopeStackItemMeta>();
        let meta_byte_offset = scope_stack.len() - meta_byte_length;
        let meta_ptr =
            scope_stack.get_mut(meta_byte_offset).unwrap() as *mut u8 as *mut ScopeStackItemMeta;
        let meta = unsafe { ptr::read(meta_ptr) };
        let ScopeStackItemMeta {
            cleanup_fn,
            frame_byte_length,
            ..
        } = meta;

        let frame_byte_offset = scope_stack.len() - frame_byte_length;
        let frame_ptr = scope_stack.get_mut(frame_byte_offset).unwrap() as *mut u8 as *mut ();
        cleanup_fn(frame_ptr, &mut self.top);

        unsafe { scope_stack.set_len(frame_byte_offset) };
    }

    #[allow(dead_code)] // False alarm.
    fn cleanup_frame<D: ScopeStackItemData>(frame_ptr: *mut (), top: &mut ScopeTop) {
        let frame_ptr = frame_ptr as *mut ScopeStackItemFrame<D>;
        let frame = unsafe { &mut *frame_ptr };
        replace(D::get_top_slot(top), frame.meta.previous_top as *mut D);
        unsafe { drop_in_place(frame.data.get()) };
    }
}

pub trait ScopeStackItemData {
    fn construct(buf: *mut Self);
    fn get_top_slot(top: &mut ScopeTop) -> &mut *mut Self;
}

struct ScopeStackItemFrame<D> {
    data: UnsafeCell<D>,
    meta: ScopeStackItemMeta,
}

struct ScopeStackItemMeta {
    previous_top: *mut (),
    cleanup_fn: fn(*mut (), &mut ScopeTop) -> (),
    frame_byte_length: usize,
}

struct For<'t>(PhantomData<&'t ()>);
type Never = std::convert::Infallible; // Forward compatible.

struct Scope<Handles = Never, Escape = Never> {
    mgr: Rc<ScopeManager>,
    cookie: u32,
    frames: u32,
    _phantom: PhantomData<(Handles, Escape)>,
}

impl Scope<Never, Never> {
    pub fn root<'a>(mgr: &'_ Rc<ScopeManager>) -> ScopeRef<'a, Never, Never> {
        let mgr = mgr.clone();
        let cookie = mgr.new_root();
        let self_ = Self {
            mgr,
            cookie,
            frames: 0,
            _phantom: PhantomData,
        };
        ScopeRef::new(self_)
    }
}

impl<'l, Escape> Scope<For<'l>, Escape> {
    pub fn with_handles<'a, Handles>(
        parent: &'a mut Scope<Handles, Escape>,
    ) -> ScopeRef<'a, For<'l>, Escape> {
        let mgr = parent.mgr.clone();
        let cookie = mgr.shadow(parent.cookie);
        mgr.get(cookie).borrow_mut().push::<HandleScopeData>();
        let self_ = Scope {
            cookie,
            mgr,
            frames: 1,
            _phantom: PhantomData,
        };
        ScopeRef::new(self_)
    }

    pub fn make_local<T>(&'_ mut self) -> Local<'l, T> {
        let _ = self.mgr.get(self.cookie); // Just check cookie.
        Default::default()
    }
}

impl<'e> Scope<For<'e>, For<'e>> {
    pub fn with_escape<'a, Escape>(
        parent: &'a mut Scope<For<'e>, Escape>,
    ) -> ScopeRef<'a, For<'e>, For<'e>> {
        let mgr = parent.mgr.clone();
        let cookie = mgr.shadow(parent.cookie);
        mgr.get(cookie).borrow_mut().push::<EscapeSlotData>();
        let self_ = Scope {
            cookie,
            mgr,
            frames: 1,
            _phantom: PhantomData,
        };
        ScopeRef::new(self_)
    }

    pub fn escape<'l, T>(&'_ mut self, local: Local<'l, T>) -> Local<'e, T> {
        let escape_slot_ptr = self.mgr.get(self.cookie).borrow_mut().top.escape_slot;
        assert!(size_of_val(&local) <= size_of::<EscapeSlotData>());
        let local_in_ptr = escape_slot_ptr as *mut Local<'l, T>;
        unsafe { ptr::write(local_in_ptr, local) };
        let local_out_ptr = escape_slot_ptr as *mut Local<'e, T>;
        unsafe { ptr::read(local_out_ptr) }
    }
}

impl<Handles, Escape> Scope<Handles, Escape> {
    pub fn _dup<'a>(parent: &'a mut Scope<Handles, Escape>) -> ScopeRef<'a, Handles, Escape> {
        let mgr = parent.mgr.clone();
        let cookie = mgr.shadow(parent.cookie);
        let self_ = Self {
            cookie,
            mgr,
            frames: 0,
            _phantom: PhantomData,
        };
        ScopeRef::new(self_)
    }
}

struct ScopeRef<'a, Handles, Escape> {
    scope: Scope<Handles, Escape>,
    _lifetime: PhantomData<&'a mut ()>,
}

impl<'a, Handles, Escape> ScopeRef<'a, Handles, Escape> {
    fn new(scope: Scope<Handles, Escape>) -> Self {
        println!("New scope: {}", std::any::type_name::<Self>());
        Self {
            scope,
            _lifetime: PhantomData,
        }
    }
}

impl<'a, Handles, Escape> Drop for ScopeRef<'a, Handles, Escape> {
    fn drop(&mut self) {
        println!("Drop scope: {}", std::any::type_name::<Self>());
        for _ in 0..self.frames {
            self.mgr.get(self.cookie).borrow_mut().pop()
        }
        self.mgr.unshadow(self.cookie)
    }
}

impl<'a, Handles, Escape> Deref for ScopeRef<'a, Handles, Escape> {
    type Target = Scope<Handles, Escape>;
    fn deref(&self) -> &Self::Target {
        &self.scope
    }
}

impl<'a, Handles, Escape> DerefMut for ScopeRef<'a, Handles, Escape> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.scope
    }
}

struct HandleScopeData([usize; 3]);
impl ScopeStackItemData for HandleScopeData {
    fn construct(buf: *mut Self) {
        unsafe { ptr::write(buf, Self(Default::default())) }
    }
    fn get_top_slot(top: &mut ScopeTop) -> &mut *mut Self {
        &mut top.handle_scope
    }
}

struct EscapeSlotData([usize; 1]);
impl ScopeStackItemData for EscapeSlotData {
    fn construct(buf: *mut Self) {
        unsafe { ptr::write(buf, Self(Default::default())) }
    }
    fn get_top_slot(top: &mut ScopeTop) -> &mut *mut Self {
        &mut top.escape_slot
    }
}
#[derive(Default)]
struct TryCatchData([usize; 7]);

pub fn testing() {
    let mgr = ScopeManager::new();
    let root = &mut Scope::root(&mgr);
    let hs = &mut Scope::with_handles(root);
    let esc = &mut Scope::with_escape(hs);
    let ehs = &mut Scope::with_handles(esc);
    let l1 = ehs.make_local::<Value>();
    let _e1 = ehs.escape(l1);
}

struct Value(*mut ());

#[derive(Copy, Clone)]
struct Local<'a, T> {
    _phantom: PhantomData<&'a T>,
    _ptr: *mut T,
}

impl<'a, T> Default for Local<'a, T> {
    fn default() -> Self {
        Local {
            _phantom: PhantomData,
            _ptr: null_mut(),
        }
    }
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

impl<'l, T> Local<'l, T> {
    fn new<'a, Escape>(scope: &'a mut Scope<For<'l>, Escape>) -> Self
    where
        'l: 'a,
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

fn indirect_make_local<'l, T, Escape>(scope: &'_ mut Scope<For<'l>, Escape>) -> Local<'l, T> {
    Local::new(scope)
}

fn use_it<T>(_: &T) {}

fn use_local<T>(_: &T) {}

struct Stuff<'a>(&'a Value, &'a Value, &'a Value);

fn main() {
    let mgr1 = ScopeManager::new();
    let root1 = &mut Scope::root(&mgr1);
    let mgr2 = ScopeManager::new();
    let root2 = &mut Scope::root(&mgr2);
    {
        let x = &mut Scope::with_handles(root1);
        let _xxv = x.make_local::<Value>();
        let yyv = {
            let mut y = Scope::with_handles(x);
            //std::mem::swap(&mut x, &mut y);
            //let r1 = Local::<Value>::new(x);
            //let r2 = (y.get_make_local())();
            let r1 = y.make_local::<Value>();
            let r2 = y.make_local::<Value>();
            let r3 = Local::<Value>::new(&mut y);
            {
                let sc = &mut Scope::root(&mgr1);
                let sc = &mut Scope::with_handles(sc);
                //let _panic = Local::<Value>::new(&mut y);
                let _scl = Local::<Value>::new(sc);
            }
            use_local(&r3);
            let r4 = Local::<Value>::new(&mut y);
            use_local(&r3);
            let g = Some(Global::<Value>::new());
            let stuff = Stuff(&r1, &r2, g.as_ref().unwrap());
            //g.replace(Global::new());
            use_local(&r1);
            use_local(g.as_ref().unwrap());
            use_it(&stuff);
            let _r5: Local<Value> = indirect_make_local(&mut y);
            let z1 = {
                let w0 = &mut Scope::with_handles(&mut y);
                let wl0 = Local::<Value>::new(w0);
                {
                    let w1 = &mut Scope::with_handles(w0);
                    let _wl1 = Local::<Value>::new(w1);
                }
                let w2 = &mut Scope::with_handles(w0);
                //let wl0x = Local::<Value>::new(w0);
                let _wl2 = Local::<Value>::new(w2);
                use_it(&r1);
                use_it(&r2);
                use_it(&r3);
                use_it(&r4);
                wl0
            };
            use_it(&z1);
            let ref mut y2 = Scope::with_handles(&mut y);
            //u = y2;
            //r
            //use_it(&z1);
            //use_it(&r5);
            //std::mem::swap(y2, y);
            let z2 = Local::<Value>::new(y2);
            let _z3 = Scope::with_handles(y2);
            use_it(&r4);
            use_it(&z2);
        };
        let _y2 = Scope::with_handles(root2);
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
