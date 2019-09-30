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
#[derive(Clone, Copy, Debug)]
pub struct Foo {
  a: i32,
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
  static AA_construct: extern "C" fn(&mut std::mem::MaybeUninit<AA>, i32, i32);
  static AA_delete: extern "C" fn(&mut AA) -> ();
  static AA_print: extern "C" fn(&mut AA, f64) -> ();
  static AA_powpow: extern "C" fn(&mut i32) -> i32;
  static BB_print: extern "C" fn(&mut BB, f64) -> ();
  static BB_get_rets: extern "C" fn(&mut BB, i32, i32, i32) -> Rets;
  static BB_print_rets: extern "C" fn(&BB, Rets, &Rets) -> ();
  static CC_construct:
    extern "C" fn(&mut std::mem::MaybeUninit<CC>, *mut [i32; 10]);
  static CC_fifth: extern "C" fn(&CC) -> &mut i32;
  static reverse_roles: extern "C" fn(&mut i32) -> ();
}

#[no_mangle]
pub extern "C" fn do_call_me_pls_rs(a: &mut i32) -> Foo {
  println!("Called back!");
  *a += 1;
  *a *= 2;
  Foo { a: 99 }
}

#[no_mangle]
pub static call_me_pls: extern "C" fn(&mut i32) -> Foo = do_call_me_pls_rs;

#[allow(unused_variables)]
fn main() {
  // let mut aa = AA {
  //  _vtable: std::ptr::null(),
  //  a_: 42,
  //};
  let mut aa = unsafe { std::mem::MaybeUninit::<AA>::uninit() };
  unsafe {
    AA_construct(&mut aa, 3, 4);
  };
  let mut aa = unsafe { aa.assume_init() };
  dbg!(&aa);
  let mut bb = BB { base: aa };
  unsafe {
    AA_print(&mut aa, 1.5f64);
    let mut a = 2i32;
    println!("Ole!");
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
    let mut cc = std::mem::MaybeUninit::<CC>::uninit();
    CC_construct(&mut cc, &mut list);
    let mut cc = cc.assume_init();
    let f = CC_fifth(&cc);
    println!("fifth {}", *f);
    let mut a = 4i32;
    reverse_roles(&mut a);
  }
}
