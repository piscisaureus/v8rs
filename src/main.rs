use test34::Context;
use test34::ContextScope;
use test34::EscapableHandleScope;
use test34::HandleScope;
use test34::Integer;
use test34::Isolate;
use test34::TryCatch;

fn main() {
  let isolate = Isolate::new();

  let mut root = HandleScope::root(&isolate);
  let root = root.enter();

  let ctx = Context::new(root);
  let mut s1 = ContextScope::new(root, &ctx);
  let s1 = s1.enter();

  let _s1l1 = Integer::new(s1, 0);
  let _s1l2 = Integer::new(s1, 0);
  let _fail = {
    let mut s2 = HandleScope::new(s1);
    let s2 = s2.enter();

    let s2l1 = Integer::new(s2, 0);
    let _s2l2 = Integer::new(s2, 0);
    //let _fail = Integer::new(s1, 0);
    s2l1
  };
  _fail;
  let _s1l3 = Integer::new(s1, 0);

  test1();
}

fn test1() {
  let isolate = Isolate::new();

  let mut root = HandleScope::root(&isolate);
  let root = root.enter();

  let ctx = Context::new(root);
  let mut s0 = ContextScope::root(&ctx);
  let s0 = s0.enter();

  let mut root2 = HandleScope::root(&ctx);
  let root2 = root2.enter();

  let mut s1 = HandleScope::new(root2);
  let s1 = s1.enter();
  let _ = Integer::new(s1, 0);

  {
    let mut s2 = HandleScope::new(s1);
    let s2 = s2.enter();
    let _ = Integer::new(s2, 0);
  }

  {
    let ctx = Context::new(s1);
    let mut s2 = ContextScope::new(s1, &ctx);
    let s2 = s2.enter();
    let _ = Integer::new(s2, 0);

    let mut s3 = HandleScope::new(s2);
    let s3 = s3.enter();
    let _ = Integer::new(s3, 0);
  }

  {
    let mut s2 = EscapableHandleScope::new(s1);
    let s2 = s2.enter();
    let _ = Integer::new(s2, 0);
    {
      let mut s3 = HandleScope::new(s2);
      let s3 = s3.enter();
      let _ = Integer::new(s3, 0);
    }
    {
      let mut s3 = TryCatch::new(s2);
      let s3 = s3.enter();
      let _ = Integer::new(s3, 0);
    }
  }

  {
    let mut s2 = TryCatch::new(s1);
    let s2 = s2.enter();
    let _ = Integer::new(s2, 0);
    {
      let mut s3 = HandleScope::new(s2);
      let s3 = s3.enter();
      let _ = Integer::new(s3, 0);
    }
    {
      let mut s3 = EscapableHandleScope::new(s2);
      let s3 = s3.enter();
      let _ = Integer::new(s3, 0);
    }
  }
}
