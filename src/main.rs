#![allow(dead_code)]

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct AA {
  _vtable: *const std::ffi::c_void,
  a_: i32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct BB {
  base: AA,
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CC {
  _a: *mut i32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Rets {
  n: [i32; 1],
  b: bool,
}

#[repr(C)]
struct class_new_0<T: 'static>(
  extern "C" fn() -> std::pin::Pin<&'static mut T>,
  extern "C" fn(&mut std::mem::MaybeUninit<T>) -> &mut T,
);
#[repr(C)]
struct class_new_1<T: 'static, A1>(
  extern "C" fn(A1) -> std::pin::Pin<&'static mut T>,
  extern "C" fn(&mut std::mem::MaybeUninit<T>, A1) -> &mut T,
);
#[repr(C)]
struct class_new_2<T: 'static, A1, A2>(
  extern "C" fn(A1, A2) -> std::pin::Pin<&'static mut T>,
  extern "C" fn(&mut std::mem::MaybeUninit<T>, A1, A2) -> (),
);

#[repr(C)]
struct class_delete<T: 'static>(
  extern "C" fn(std::pin::Pin<&'static mut T>) -> (),
  extern "C" fn(&mut T) -> &mut std::mem::MaybeUninit<T>,
);

extern "C" {
  static AA_new: class_new_1<AA, i32>;
  static AA_delete: class_delete<AA>;
  static AA_print: extern "C" fn(&mut AA, f64) -> ();
  static AA_powpow: extern "C" fn(&mut i32) -> i32;
  static BB_print: extern "C" fn(&mut BB, f64) -> ();
  static BB_get_rets: extern "C" fn(&mut BB, i32, i32, i32) -> Rets;
  static BB_print_rets: extern "C" fn(&BB, Rets, &Rets) -> ();
  static CC_new: class_new_1<CC, *mut [i32; 10]>;
  static CC_fifth: extern "C" fn(&CC) -> &mut i32;
}

fn main() {
  // let mut aa = AA {
  //  _vtable: std::ptr::null(),
  //  a_: 42,
  //};
  let mut aa = unsafe { AA_new.0(99) };
  dbg!(&aa);
  let mut bb = BB { base: *aa };
  unsafe {
    AA_print(&mut aa, 1.5f64);
    let mut a = 2i32;
    let aa = AA_powpow(&mut a);
    println!("AA_powpow: a^2={} a^2^2={}", a, aa);
    BB_print(&mut bb, 2.5f64);
    let r1 = BB_get_rets(&mut bb, 1, 2, 3);
    dbg!(r1);
    let r2 = Rets { n: [-42], b: true };
    BB_print_rets(&bb, r1, &r2);
  }

  unsafe {
    let mut list: [i32; 10] = [0; 10];
    for i in 0..9 {
      list[i] = i as i32;
    }
    let mut cc = std::mem::MaybeUninit::uninit();
    let mut cc = CC_new.1(&mut cc, &mut list);
    let mut f = CC_fifth(&cc);
    println!("fifth {}", *f);
  }
}
