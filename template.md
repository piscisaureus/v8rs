
// Existing

```cpp
namespace ns {
  struct SomeData { 
    int field;
  };

  class SomeClass {
  public:
    SomeClass(int arg);
    virtual ~SomeClass();

    static int StaticMethod1();
    static void StaticMethod2(int arg);
    
    void MutMethod();
    int ConstMethod(int arg) const;

    SomeData MethodReturningObject();

    SomeData& MethodWithRefs(const SomeClass& arg);
    unique_ptr<SomeData> MethodWithSmartPtrs(unique_ptr<const SomeData> arg);
    
    virtual int VirtualMethod(int arg);
  };
}
```

// Utility

```cpp
namespace c_abi {
  template<class T> class uninit_t { /* ... */ } 
  template<class T> class return_t { /* ... */ } 
}
```

// C ABI wrapper

```cpp
extern "C" {
  void ns__SomeClass__CTOR(::c_abi::uninit_t<ns__SomeClass>& self, int arg);
  void ns__SomeClass__DTOR(SomeClass& self);

  int ns__SomeClass__StaticMethod1(void);
  void ns__SomeClass__StaticMethod2(int arg);

  void ns__SomeClass__MutMethod(ns__SomeClass& self);
  int ns__SomeClass__ConstMethod(const ns__SomeClass& self, int arg);

  ::c_abi::return_t<ns__SomeData> ns__SomeClass__MethodReturningObject(ns__SomeClass& self);

  ns__SomeData& ns__SomeClass__MethodWithRefs(ns__SomeClass& self, const ns__SomeClass& arg);
  ns__SomeData* ns__SomeClass__MethodWithSmartPtrs(ns__SomeClass& self, const ns__SomeData* arg);

  int ns__SomeClass__VirtualMethod(ns__SomeClass& self, int arg);
  int ns__SomeClass__VirtualMethod__BASE(ns__SomeClass& self, int arg);
}
```

```rust
extern "C" {
  fn ns__SomeClass__CTOR(this: &mut std::mem::MaybeUninit<ns::SomeClass> self, arg: i32) -> ();
  fn ns__SomeClass__DTOR(this: &mut SomeClass) -> ();

  fn ns__SomeClass__StaticMethod1() -> i32;
  fn ns__SomeClass__StaticMethod2(arg: i32) -> ();

  fn ns__SomeClass__MutMethod(this: &mut ns::SomeClass) -> ();
  fn ns__SomeClass__ConstMethod(this: &ns::SomeClass self, arg: i32) -> i32;

  fn ns__SomeClass__MethodReturningObject(this: &mut ns::SomeClass) -> ns::SomeData;

  fn ns__SomeClass__MethodWithRefs(this: &mut ns::SomeClass, arg: &ns::SomeClass) -> &ns::SomeData;
  fn ns__SomeClass__MethodWithSmartPtrs(this: &mut ns::SomeClass, arg: &ns::SomeData) -> *mut ns::SomeData;

  fn ns__SomeClass__VirtualMethod(this: &mut ns::SomeClass, arg: i32) -> i32;
  fn ns__SomeClass__VirtualMethod__BASE(this: &mut ns::SomeClass, arg: i32) -> i32;
}
```

```rust
mod ns {
  pub struct SomeData { 
    pub field: i32;
  };

  pub struct SomeClass {}

  impl SomeClass {
    pub fn new(arg: i32) -> Self {
      let mut this = std::mem::MaybeUninit::<T>();
      ns__SomeClass__CTOR(&mut this);
      this.assume_init()
    }
  }

  impl std::ops::Drop for SomeClass {
    fn drop(&mut self) -> () {
      ns__SomeClass__DTOR(self);
      std::mem::forget(self);
    }
  }

  impl SomeClass {
    pub fn StaticMethod1() -> i32 { ns__SomeClass__StaticMethod1() }
    pub fn StaticMethod2(arg: i32) -> () { ns__SomeClass__StaticMethod2(arg) }
    
    pub fn MutMethod(&mut self) -> () { ns__SomeClass__MutMethod(self) }
    pub fn ConstMethod(&self, arg: i32) -> i32 { ns__SomeClass__ConstMethod(self, arg) }

    pub fn MethodReturningObject(&mut self) -> ns::SomeData { ns__SomeClass__MethodReturningObject(self) }

    pub fn MethodWithRefs(&mut self, arg: &ns::SomeClass) -> &ns::SomeData { ns__SomeClass__MethodWithRefs(self, arg) }
    pub fn MethodWithSmartPtrs(&mut self, arg: &ns::SomeData) -> *mut ns::SomeData { ns__SomeClass__MethodWithSmartPtrs(self, arg) }

    pub fn VirtualMethod(&mut self, arg: i32) -> i32 { ns__SomeClass__VirtualMethod(self, arg) }
    fn VirtualMethod__DEFAULT(&mut self, arg: i32) -> i32 { ns__SomeClass__VirtualMethod__DEFAULT(self, arg) }
  }
}
```