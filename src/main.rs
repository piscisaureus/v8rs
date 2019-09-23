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
  static AA_powpow: extern "C" fn(&mut i32) -> i32;
  static BB_print: extern "C" fn(&mut BB, f64) -> ();
  static BB_get_rets: extern "C" fn(&mut BB, i32, i32, i32) -> Rets;
  static BB_print_rets: extern "C" fn(&mut BB, Rets, &Rets) -> ();
}

fn main() {
  let mut aa = AA {
    _vtable: std::ptr::null(),
    a_: 42,
  };
  let mut bb = BB { base: aa };
  unsafe {
    AA_print(&mut aa, 1.5f64);
    let mut a = 2i32;
    let aa = AA_powpow(&mut a);
    println!("AA_powpow: a^2={} a^2^2={}", a, aa);
    BB_print(&mut bb, 2.5f64);
    let r1 = BB_get_rets(&mut bb, 1, 2, 3);
    dbg!(r1);
    let r2 = Rets { n: [-42], b: true };
    BB_print_rets(&mut bb, r1, &r2);
  }
}
