mod channel {
    use super::*;

    #[repr(C)]
    pub struct Channel {
        _cxx_vtable: *const [usize; 0],
    }

    #[repr(C)]
    pub struct Override<'a> {
        _cxx_vtable: *const [usize; 0],
        rs_trait_obj: &'a mut dyn OverrideMethods<'a>,
    }

    pub trait DirectDispatchMethods {
        fn a(&mut self) -> ();
        fn b(&self) -> ();
    }

    pub trait OverrideMethods<'a>: AsRef<Override<'a>> + AsMut<Override<'a>> {
        fn a(&mut self) -> () {
            let o: &mut Override<'a> = self.as_mut();
            let c: &mut Channel = o.as_mut();
            <Channel as DirectDispatchMethods>::a(o)
        }
        fn b(&self) -> ();
    }

    extern "C" {
        fn Channel__DTOR(this: &mut Channel) -> ();
        fn Channel__a(this: &mut Channel) -> ();
        fn Channel__a__a(this: &mut Channel) -> ();
        fn Channel__b(this: &Channel) -> ();

        fn Channel__OVERRIDE__CTOR(this: &mut std::mem::MaybeUninit<Override>) -> ();
        fn Channel__OVERRIDE__DTOR(this: &mut Override) -> ();
    }

    #[no_mangle]
    extern "C" fn Channel__OVERRIDE__a__DISPATCH(this: &mut Override) -> () {
        {
            this.rs_trait_obj.a()
        }
    }
    #[no_mangle]
    extern "C" fn Channel__OVERRIDE__b__DISPATCH(this: &Override) -> () {
        this.rs_trait_obj.b()
    }

    impl Channel {
        pub fn a(&mut self) -> () {
            unsafe { Channel__a(self) }
        }
        pub fn b(&self) -> () {
            unsafe { Channel__b(self) }
        }
    }

    impl DirectDispatchMethods for Channel {
        fn a(&mut self) -> () {
            unsafe { Channel__a__a(self) }
        }
        fn b(&self) -> () {
            panic!("pure virtual function call")
        }
    }

    impl Drop for Channel {
        fn drop(&mut self) -> () {
            unsafe { Channel__DTOR(self) }
        }
    }

    impl<'a> Override<'a> {
        pub fn new(implementer: &'a mut dyn OverrideMethods<'a>) -> Self {
            let mut mem = std::mem::MaybeUninit::<Self>::uninit();
            unsafe {
                Channel__OVERRIDE__CTOR(&mut mem);
                let p = mem.as_mut_ptr();
                let p: *mut &'a mut dyn OverrideMethods<'a> = &mut ((*p).rs_trait_obj);
                p.write(implementer);
                mem.assume_init()
            }
        }
    }

    impl<'a> DirectDispatchMethods for Override<'a> {
        fn a(&mut self) -> () {
            Channel__OVERRIDE__a__DISPATCH(self)
        }
        fn b(&self) -> () {
            Channel__OVERRIDE__b__DISPATCH(self)
        }
    }

    impl<'a> Drop for Override<'a> {
        fn drop(&mut self) {
            unsafe { Channel__OVERRIDE__DTOR(self) }
        }
    }

    impl<'a> std::ops::Deref for Override<'a> {
        type Target = Channel;
        fn deref(&self) -> &Channel {
            unsafe { std::mem::transmute(self) }
        }
    }

    impl<'a> std::ops::DerefMut for Override<'a> {
        fn deref_mut(&mut self) -> &mut Channel {
            unsafe { std::mem::transmute(self) }
        }
    }

    impl<'a> AsRef<Channel> for Override<'a> {
        fn as_ref(&self) -> &Channel {
            &*self
        }
    }

    impl<'a> AsMut<Channel> for Override<'a> {
        fn as_mut(&mut self) -> &mut Channel {
            &mut *self
        }
    }
}

mod tries {
    use super::channel::*;

    struct Session<'a> {
        a: i32,
        b: String,
        c: Option<Override<'a>>,
    }

    impl<'a> AsRef<Override<'a>> for Session<'a> {
        fn as_ref(&self) -> &Override<'a> {
            self.c.as_ref().unwrap()
        }
    }

    impl<'a> AsMut<Override<'a>> for Session<'a> {
        fn as_mut(&mut self) -> &mut Override<'a> {
            self.c.as_mut().unwrap()
        }
    }

    impl<'a> OverrideMethods<'a> for Session<'a> {
        fn a(&mut self) {
            println!("Override a!");
        }
        fn b(&self) {
            println!("Override b!");
        }
    }

    impl<'a> Session<'a> {
        pub fn new() -> Self {
            let mut s = Self {
                a: 1,
                b: "abc".to_owned(),
                c: None,
            };
            s.c.replace(Override::new(&mut s));
            s
        }
    }
}
