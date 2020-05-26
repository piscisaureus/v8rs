use test34::Context;
use test34::EscapableHandleScope;
use test34::HandleScope;
use test34::Local;
use test34::TryCatch;

fn main() {
  let mut root = HandleScope::root();
  let _root = root.enter();

  let mut ctx = Context::new();

  let mut s1 = HandleScope::new(&mut ctx);
  let s1 = s1.enter();

  let _s1l1 = Local::<i8>::new(s1);
  let _s1l2 = Local::<i8>::new(s1);
  let _fail = {
    let mut s2 = HandleScope::new(s1);
    let s2 = s2.enter();

    let s2l1 = Local::<i8>::new(s2);
    let _s2l2 = Local::<i8>::new(s2);
    //let _fail = Local::<i8>::new(s1);
    s2l1;
  };
  let _s1l3 = Local::<i8>::new(s1);

  test1();
}

fn test1() {
  let mut ctx = Context::new();

  let mut s1 = HandleScope::new(&mut ctx);
  let s1 = s1.enter();
  let _ = Local::<i8>::new(s1);

  {
    let mut s2 = HandleScope::new(s1);
    let s2 = s2.enter();
    let _ = Local::<i8>::new(s2);
  }

  {
    let mut s2 = EscapableHandleScope::new(s1);
    let s2 = s2.enter();
    let _ = Local::<i8>::new(s2);
    {
      let mut s3 = HandleScope::new(s2);
      let s3 = s3.enter();
      let _ = Local::<i8>::new(s3);
    }
    {
      let mut s3 = TryCatch::new(s2);
      let s3 = s3.enter();
      let _ = Local::<i8>::new(s3);
    }
  }

  {
    let mut s2 = TryCatch::new(s1);
    let s2 = s2.enter();
    let _ = Local::<i8>::new(s2);
    {
      let mut s3 = HandleScope::new(s2);
      let s3 = s3.enter();
      let _ = Local::<i8>::new(s3);
    }
    {
      let mut s3 = EscapableHandleScope::new(s2);
      let s3 = s3.enter();
      let _ = Local::<i8>::new(s3);
    }
  }
}
