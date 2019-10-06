mod c_abi {}

mod util {
    use std::marker::PhantomData;
    use std::mem::{size_of, MaybeUninit};

    pub type Opaque = [usize; 0];

    #[repr(transparent)]
    #[derive(Copy, Clone, Debug)]
    pub struct RustVTable<DynT>(pub *const Opaque, pub PhantomData<DynT>);

    #[derive(Copy, Clone, Debug)]
    #[repr(transparent)]
    pub struct FieldOffset<O, I>(isize, PhantomData<(O, I)>);

    impl<O, I> FieldOffset<O, I> {
        pub fn from_ptrs(o_ptr: *const O, i_ptr: *const I) -> Self {
            let o_addr = o_ptr as usize;
            let i_addr = i_ptr as usize;
            assert!(i_addr >= o_addr);
            assert!((i_addr + size_of::<I>()) <= (o_addr + size_of::<O>()));
            let offset = (o_addr - i_addr) as isize;
            assert!(offset > 0);
            Self(offset, PhantomData)
        }
        pub fn from_offset(offset: usize) -> Self {
            assert!((offset as isize) > 0);
            Self(offset as isize, PhantomData)
        }

        pub fn offset(self) -> usize {
            self.0 as usize
        }

        fn shift<PI, PO>(ptr: *const PI, delta: isize) -> *mut PO {
            (ptr as isize + delta) as *mut PO
        }
        pub unsafe fn to_outer(self, inner: &I) -> &O {
            Self::shift::<I, O>(inner, -self.0).as_ref().unwrap()
        }
        #[allow(dead_code)]
        pub unsafe fn to_outer_mut(self, inner: &mut I) -> &mut O {
            Self::shift::<I, O>(inner, -self.0).as_mut().unwrap()
        }
    }

    impl<O, M, I> std::ops::Add<FieldOffset<M, I>> for FieldOffset<O, M> {
        type Output = FieldOffset<O, I>;
        fn add(self, that: FieldOffset<M, I>) -> Self::Output {
            FieldOffset::<O, I>::from_offset(self.offset() + that.offset())
        }
    }
}

mod channel {
    use super::util;

    #[repr(C)]
    pub struct Channel {
        _cxx_vtable: *const [usize; 0],
    }

    #[repr(C)]
    pub struct Override {
        cxx_base_channel: Channel,
        cxx_base_offset: usize,
        rust_vtable: util::RustVTable<&'static dyn OverrideMethods>,
    }

    pub trait DirectDispatchMethods {
        fn a(&mut self) -> ();
        fn b(&self) -> ();
    }

    pub trait OverrideMethods {
        fn channel(&self) -> &Channel;
        fn channel_mut(&mut self) -> &mut Channel;

        fn a(&mut self) -> () {
            let channel = self.channel_mut();
            <Channel as DirectDispatchMethods>::a(channel)
        }
        fn b(&self) -> ();
    }

    extern "C" {
        fn Channel__DTOR(this: &mut Channel) -> ();
        fn Channel__a(this: &mut Channel) -> ();
        fn Channel__a__a(this: &mut Channel) -> ();
        fn Channel__b(this: &Channel) -> ();

        fn Channel__OVERRIDE__CTOR(this: &mut std::mem::MaybeUninit<Channel>) -> ();
        fn Channel__OVERRIDE__DTOR(this: &mut Channel) -> ();
    }

    #[no_mangle]
    unsafe extern "C" fn Channel__OVERRIDE__a__DISPATCH(this: &mut Channel) -> () {
        {
            Override::dispatch_mut(this).a()
        }
    }
    #[no_mangle]
    unsafe extern "C" fn Channel__OVERRIDE__b__DISPATCH(this: &Channel) -> () {
        Override::dispatch(this).b()
    }

    impl DirectDispatchMethods for Channel {
        fn a(&mut self) -> () {
            unsafe { Channel__a__a(self) }
        }
        fn b(&self) -> () {
            panic!("pure virtual function call")
        }
    }

    impl Channel {
        pub fn a(&mut self) -> () {
            unsafe { Channel__a(self) }
        }
        pub fn b(&self) -> () {
            unsafe { Channel__b(self) }
        }
    }

    impl Drop for Channel {
        fn drop(&mut self) -> () {
            unsafe { Channel__DTOR(self) }
        }
    }

    impl Override {
        fn make_cxx_base_channel() -> Channel {
            unsafe {
                let mut buf = std::mem::MaybeUninit::<Channel>::uninit();
                Channel__OVERRIDE__CTOR(&mut buf);
                buf.assume_init()
            }
        }

        fn get_cxx_base_offset<T>() -> usize
        where
            T: OverrideMethods,
        {
            let buf = std::mem::MaybeUninit::<T>::uninit();
            let top_ptr: *const T = buf.as_ptr();
            let channel_ptr: *const Channel = unsafe { (*top_ptr).channel() };
            util::FieldOffset::from_ptrs(top_ptr, channel_ptr).offset()
        }

        fn get_rust_vtable<T>() -> util::RustVTable<&'static dyn OverrideMethods>
        where
            T: OverrideMethods,
        {
            let buf = std::mem::MaybeUninit::<T>::uninit();
            let embedder_ptr = buf.as_ptr();
            let trait_object: *const dyn OverrideMethods = embedder_ptr;
            let (data_ptr, vtable): (*const T, util::RustVTable<_>) =
                unsafe { std::mem::transmute(trait_object) };
            assert_eq!(data_ptr, embedder_ptr);
            vtable
        }

        pub fn new<T>() -> Self
        where
            T: OverrideMethods,
        {
            Self {
                cxx_base_channel: Self::make_cxx_base_channel(),
                cxx_base_offset: Self::get_cxx_base_offset::<T>(),
                rust_vtable: Self::get_rust_vtable::<T>(),
            }
        }

        unsafe fn get_self(channel: &Channel) -> &Self {
            let buf = std::mem::MaybeUninit::<Self>::uninit();
            let offset =
                util::FieldOffset::from_ptrs(buf.as_ptr(), &(*buf.as_ptr()).cxx_base_channel);
            offset.to_outer(channel)
        }

        unsafe fn make_trait_object(&self) -> &mut dyn OverrideMethods {
            use util::Opaque as Embedder;
            let vtable = self.rust_vtable;
            let offset = util::FieldOffset::<Embedder, Channel>::from_offset(self.cxx_base_offset);
            let embedder_ptr = offset.to_outer(&self.cxx_base_channel);
            std::mem::transmute((embedder_ptr, vtable))
        }

        unsafe fn dispatch(channel: &Channel) -> &dyn OverrideMethods {
            Self::get_self(channel).make_trait_object()
        }
        unsafe fn dispatch_mut(channel: &mut Channel) -> &mut dyn OverrideMethods {
            Self::get_self(channel).make_trait_object()
        }
    }

    impl std::ops::Deref for Override {
        type Target = Channel;
        fn deref(&self) -> &Channel {
            &self.cxx_base_channel
        }
    }

    impl std::ops::DerefMut for Override {
        fn deref_mut(&mut self) -> &mut Channel {
            &mut self.cxx_base_channel
        }
    }
}

mod trying {
    use super::channel::*;

    pub struct Session {
        a: i32,
        b: String,
        c: Override,
    }

    impl OverrideMethods for Session {
        fn channel(&self) -> &Channel {
            &self.c
        }
        fn channel_mut(&mut self) -> &mut Channel {
            &mut self.c
        }
        fn a(&mut self) {
            println!("Override a!");
        }
        fn b(&self) {
            println!("Override b!");
        }
    }

    impl Session {
        pub fn new() -> Self {
            let s = Self {
                a: 1,
                b: "abc".to_owned(),
                c: Override::new::<Self>(),
            };
            s
        }
    }
}

fn main() {
    let s = trying::Session::new();
}
