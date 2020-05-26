use test34::Context;
use test34::ContextScope;
use test34::EscapableHandleScope;
use test34::HandleScope;
use test34::Local;
use test34::TryCatch;

fn main() {
  let mut root = HandleScope::root();
  let root = root.enter();

  let ctx = Context::new();
  let mut s1 = ContextScope::new(root, &ctx);
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
  let ctx = Context::new();
  let mut s0 = ContextScope::root(&ctx);
  let s0 = s0.enter();

  let mut s1 = HandleScope::new(s0);
  let s1 = s1.enter();
  let _ = Local::<i8>::new(s1);

  {
    let mut s2 = HandleScope::new(s1);
    let s2 = s2.enter();
    let _ = Local::<i8>::new(s2);
  }

  {
    let ctx = Context::new();
    let mut s2 = ContextScope::new(s1, &ctx);
    let s2 = s2.enter();
    let _ = Local::<i8>::new(s2);

    let mut s3 = HandleScope::new(s2);
    let s3 = s3.enter();
    let _ = Local::<i8>::new(s3);
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
