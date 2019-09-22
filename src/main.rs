#[repr(C)]
#[derive(Clone, Copy)]
struct AA {
    _vtable: *const std::ffi::c_void,
    a_: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct BB {
    base: AA,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Rets {
    n: [i32; 1],
    b: bool,
}

extern "C" {
    static AA_print: extern "C" fn(&mut AA, f64) -> ();
    static BB_print: extern "C" fn(&mut BB, f64) -> ();
    static BB_get_rets: extern "C" fn(&mut BB, i32, i32, i32) -> Rets;
    static BB_print_rets: extern "C" fn(&mut BB, &Rets) -> ();
}

fn main() {
    let mut aa = AA {
        _vtable: std::ptr::null(),
        a_: 42,
    };
    let mut bb = BB { base: aa };
    unsafe {
        AA_print(&mut aa, 1.5f64);
        BB_print(&mut bb, 2.5f64);
        let r = BB_get_rets(&mut bb, 1, 2, 3);
        dbg!(r);
        BB_print_rets(&mut bb, &r);
    }
}
