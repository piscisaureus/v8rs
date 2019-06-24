

struct Isolate {
    
}

struct Global<'a> {
    x: std::marker::PhantomData<(&'a Isolate)>
}

impl<'a> Global<'a> {
    fn new(i: &'a Isolate) -> Self {
        unimplemented!()
    }

    fn get<'b>(&'a self, i: &'b Isolate) -> i32 where 'a: 'b, 'b: 'a {
        unimplemented!()
    }
}




fn main() {
  let is1 = Isolate {};
  {
  let is2 = Isolate {};
  let g1 = Global::new(&is1);
  let g2 = Global::new(&is2);
  g1.get(&is1);
  g1.get(&is2);
  g2.get(&is1);
  g2.get(&is2);
  }
}