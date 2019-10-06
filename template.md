## Existing API (e.g. defined in v8.h)

```cpp
namespace v8 {
  // POD struct with public data members.
  struct SomeData {
    int field;
  };

  // V8-implemented class.
  class SomeClass {
    int private_field_1;
    void* private_field_2;

  public:
    SomeClass(int arg);
    virtual ~SomeClass();

    static int StaticMethod1();
    static void StaticMethod2(int arg);

    void MutMethod();
    int ConstMethod(int arg) const;

    SomeData MethodReturningObject(int arg);

    SomeData& MethodWithRefs(const SomeClass& arg);
    unique_ptr<SomeData> MethodWithSmartPtrs(unique_ptr<const SomeData> arg);
  };

  // There are a few v8 classes that have virtual methods which the embedder
  // is supposed to override.
  class UserClass {
    int private_field_1;
    void* private_field_2;

  public:
    virtual ~UserClass() = default;

    virtual int VirtualMethod(int arg);
    virtual size_t PureVirtualMethod() const = 0;
  };
}
```

## Utilities

```cpp
namespace c_abi {
  // Struct that represents a memory location suitable for holding an object
  // of type T, but which is currently uninitialized. Similar to Rust's
  // `std::mem::MaybeUninit::<T>`.
  template <class T>
  struct uninit_t: ::std::aligned_storage<sizeof(T), alignof(T)> {};
}
```

## C++/C ABI wrapper

```cpp
extern "C" {
  void v8__SomeClass__CTOR(::c_abi::uninit_t<::v8::SomeClass>& self, int arg) {
    // Note: placement new might not work for some V8 classes that deliberately
    // restrict the use of new. There's an alternative solution using a union
    // that solves the same problem.
    new (&self) ::v8::SomeClass(arg);
  }
  void v8__SomeClass__DTOR(::v8::SomeClass& self) {
    self.~SomeClass();
  }

  int v8__SomeClass__StaticMethod1(void) {
    return ::v8::SomeClass::StaticMethod1;
  }
  void v8__SomeClass__StaticMethod2(int arg) {
    ::v8::SomeClass::StaticMethod2(arg);
  }

  void v8__SomeClass__MutMethod(::v8::SomeClass& self) {
    self.MutMethod();
  }
  int v8__SomeClass__ConstMethod(const ::v8::SomeClass& self, int arg) {
    return self.ConstMethod(arg);
  }

  // When a C++ object is returned from a function, the way this is done depends
  // on complicated details (e.g. whether the class has custom constructors,
  // has a trivial destructor, standard layout, etc). In Rust you cannot
  // specify how C++ expects it to be done. Therefore we use an out param to
  // return objects instead.
  void v8__SomeClass__MethodReturningObject(
      ::v8::SomeClass& self,
      int arg,
      ::c_abi::uninit_t<v8::SomeData>& ret) {
    new (&ret) ::v8::SomeData(self.MethodReturningObject(arg));
  }

  // Although references do not actually exist in C, they're valid in
  // `extern "C"` function defintions in both C++ and Rust. From an ABI
  // perspective, they're equivalent to non-null pointers.
  ::v8::SomeData& v8__SomeClass__MethodWithRefs(::v8::SomeClass& self,
                                                const ::v8::SomeClass& arg) {
    return self.MethodWithRefs(arg);
  }

  // Smart pointers are unboxed before crossing over to the other language.
  // On the other end, they should be appropriately re-boxed to ensure that
  // the correct destructor is called (like we currently do witn PinnedBuf).
  ::v8::SomeData* v8__SomeClass__MethodWithSmartPtrs(
      ::v8::SomeClass& self,
      const ::v8::SomeData* arg) {
    return self.MethodWithSmartPtrs(
        ::std::unique_ptr<const ::v8::SomeData>(arg)).release();
  }

  int v8__UserClass__VirtualMethod(::v8::UserClass& self, int arg) {
    return self.VirtualMethod(arg);
  }
  // If the embedder chooses not to override a virtual method (assuming it's
  // not a pure virtual method), it must have a way to call the default
  // implementation specified by the base class.
  int v8__UserClass__UserClass__VirtualMethod(::v8::UserClass& self, int arg) {
    return self.::v8::UserClass:VirtualMethod(arg);
  }
  size_t v8__UserClass__PureVirtualMethod(const ::v8::UserClass& self) {
    return self.PureVirtualMethod();
  }

  void DERIVED__v8__UserClass__CTOR(::c_abi::uninit_t<::DERIVED::v8::UserClass>& self) {
    new (&self) ::v8::SomeClass(arg);
  }
}

namespace DERIVED {
namespace v8 {
  class UserClass;
}
}

extern "C" {
  size_t OVERRIDE__v8__UserClass__DTOR(
      ::DERIVED::v8::UserClass& self);
  size_t OVERRIDE__v8__UserClass__DELETE(
      ::c_abi::uninit_t<::DERIVED::v8::UserClass>& self);
  int OVERRIDE__v8__UserClass__VirtualMethod(
      ::DERIVED::v8::UserClass& self, int arg);
  size_t OVERRIDE__v8__UserClass__PureVirtualMethod(
      const ::DERIVED::v8::UserClass&);
}

namespace DERIVED {
namespace v8 {
  // The actual size of classes that are extended in rust is not known in C++,
  // rust derivate may add any number of data fields to it. Therefore they
  // should never be constructed in C++.

  class UserClass: final public ::v8::UserClass {
    // Prohibit construction from C++.
    UserClass() = delete;

  public:
    virtual ~UserClass() {
      OVERRIDE__v8__UserClass__DTOR(*this);
    };

    static void operator delete(void* ptr) {
      auto& mem = *reinterpret_cast<::c_abi::uninit_t<UserClass>*>(ptr);
      OVERRIDE__v8__UserClass__DELETE(mem);
    }

    int VirtualMethod(int arg) override {
      return OVERRIDE__v8__UserClass__VirtualMethod(*self, arg);
    }

    size_t PureVirtualMethod() const override {
      return OVERRIDE__v8__UserClass__PureVirtualMethod(*self);
    }
  };
}
}
```

## Rust/C ABI wrapper

```rust
extern "C" {
  fn v8__SomeClass__CTOR(this: &mut std::mem::MaybeUninit::<v8::SomeClass> self, arg: i32) -> ();
  fn v8__SomeClass__DTOR(this: &mut v8::SomeClass) -> ();

  fn v8__SomeClass__StaticMethod1() -> i32;
  fn v8__SomeClass__StaticMethod2(arg: i32) -> ();

  fn v8__SomeClass__MutMethod(this: &mut v8::SomeClass) -> ();
  fn v8__SomeClass__ConstMethod(this: &v8::SomeClass self, arg: i32) -> i32;

  fn v8__SomeClass__MethodReturningObject(this: &mut v8::SomeClass, arg: i32, ret: &mut std::mem::MaybeUninit::<v8::SomeClass>) -> ();

  fn v8__SomeClass__MethodWithRefs(this: &mut v8::SomeClass, arg: &v8::SomeClass) -> &v8::SomeData;
  fn v8__SomeClass__MethodWithSmartPtrs(this: &mut v8::SomeClass, arg: &v8::SomeData) -> *mut v8::SomeData;

  fn v8__UserClass__VirtualMethod(this: &mut v8::UserClass, arg: i32) -> i32;
  fn v8__UserClass__UserClass__VirtualMethod(this: &mut v8::UserClass, arg: i32) -> i32;
  fn v8__UserClass__PureVirtualMethod(this: &v8::UserClass& self) -> usize;

  fn DERIVED__v8__UserClass__CTOR(this: &mut std::mem::MaybeUninit<v8::user_class::Derived>);
  fn DERIVED__v8__UserClass__DTOR(this: &mut v8::user_class::Derived);
}
```

## Rust high-level wrapper.

```rust
mod v8 {
  pub struct SomeData {
    pub field: i32;
  };

  pub use some_class::SomeClass;
  pub mod some_class {
    pub use super::*;

    pub struct SomeClass([usize; 2]);

    impl SomeClass {
      pub fn new(arg: i32) -> Self {
        let mut this = std::mem::MaybeUninit::<T>::uninit();
        v8__SomeClass__CTOR(&mut this);
        this.assume_init()
      }
    }

    impl std::ops::Drop for SomeClass {
      fn drop(&mut self) -> () {
        v8__SomeClass__DTOR(self);
      }
    }

    impl SomeClass {
      pub fn StaticMethod1() -> i32 {
        v8__SomeClass__StaticMethod1()
      }
      pub fn StaticMethod2(arg: i32) -> () {
        v8__SomeClass__StaticMethod2(arg)
      }

      pub fn MutMethod(&mut self) -> () {
        v8__SomeClass__MutMethod(self)
      }
      pub fn ConstMethod(&self, arg: i32) -> i32 {
        v8__SomeClass__ConstMethod(self, arg)
      }

      pub fn MethodReturningObject(&mut self, arg: i32) -> v8::SomeData {
        let mut ret = std::mem::maybeUninit::<v8::SomeData>::uninit();
        v8__SomeClass__MethodReturningObject(self, arg, &mut ret);
        ret.assume_init()
      }

      pub fn MethodWithRefs(&mut self, arg: &v8::SomeClass) -> &v8::SomeData {
        v8__SomeClass__MethodWithRefs(self, arg)
      }
      pub fn MethodWithSmartPtrs(&mut self, arg: UniquePtr<v8::SomeData>)
          -> UniquePtr<v8::SomeData> {
        v8__SomeClass__MethodWithSmartPtrs(self, arg.into_raw()).into()
      }
    }
  }

  pub use user_class::UserClass;
  pub mod user_class {
    pub use super::*;

    #[repr(C)]
    pub struct UserClass([usize; 3]);

    impl UserClass {
      pub fn VirtualMethod(&mut self, arg: i32) -> i32 {
        v8__UserClass__VirtualMethod(self, arg)
      }
      pub fn PureVirtualMethod(&self) -> usize {
        v8__UserClass__PureVirtualMethod(self)
      }
    }

    impl std::ops::Drop for UserClass {
      fn drop(&mut self) -> () {
        v8__SomeClass__DTOR(self);
      }
    }

    pub type Derived = Extend<[u8; 0]>;

    pub struct Extend where Self: Override {
      base: UserClass,
      overrides: ManuallyDrop<*const dyn Override>,
      data: ManuallyDrop<<Self as Override>::Data>
    };

    struct ExtendUninit<T> {
      use std::mem::{MaybeUninit, ManuallyDrop>;
      base: MaybeUninit<UserClass>,
      overrides: MaybeUninit<ManuallyDrop<*const dyn Override>>,
      data: MaybeUnininit<ManuallyDrop<T>>
    }

    impl ExtendUninit<T> {
      use std::mem::MaybeUninit;
      fn new() -> Self {
        Self {
          base: MaybeUninit::uninit(),
          vtable: MaybeUninit::uninit(),
          data: MaybeUninit::uninit(),
        }
      }
      unsafe fn init<FnB, FnV, FnD>(&mut self, b: FnV, v: FnO, d: FnD) -> () where
        FnB: FnOnce(&mut MaybeUninit<UserClass>) -> (),
        FnV: FnOnce(&mut MaybeUninit<*const dyn Override>) -> (),
        FnD: FnOnce(&mut MaybeUninit<T>) -> ()> {
          b(&mut self.base);
          v(&mut self.vtable);
          d(&mut self.data)
        }
    }

    impl Extend where Self: Override {
      pub fn base(&self) -> &UserClass {
        &self.base
      }
      pub fn base_mut(&self) -> &mut UserClass {
        &mut self.base
      }
      pub fn new(data: T) -> Box<Self> {
        let alloc = Box::new(ExtendMaybeUninit::<Self::Data>::new());
        let vtable: *const dyn Override = unsafe {
          let temp: &Self = std::mem::transmute(alloc);
          let temp: &dyn Override = temp;
          std::mem::transmute(temp)
        };
        mem.init(|m| DERIVED__v8__UserClass__CTOR(m),
                 |m| *m = MaybeUninit::new(vtable),
                 |m| *m = MaybeUninit::new(data));
        unsafe { std::mem::transmute() }
      }
      fn dtor(&mut self) {

      }
    }

    impl std::ops::Deref for Extend {
      type Target = <Extend as Override>::Data;
      fn deref(&self) -> &Self::Target {
        &self.data
      }
    }

    impl std::ops::DerefMut for Extend {
      fn deref_mut(&self) -> &mut <Self as Deref>::Target {
        &self.data
      }
    }

    pub trait Override where Self: AsRef<Extend> {
      pub type Data;

      pub fn dtor(&mut self) -> () {
        ManuallyDrop::drop(&mut self.data);
        ManuallyDrop::drop(&mut self.overrides);
      }

      pub fn VirtualMethod(&mut self, arg: i32) -> i32 {
        v8__UserClass__UserClass__VirtualMethod(self.base_mut(), arg)
      }

      pub fn PureVirtualMethod(&self) -> usize {
        v8__UserClass__PureVirtualMethod(self)
      }
    }

    #[no_mangle]
    extern "C" fn OVERRIDE__v8__UserClass__DTOR(this: &mut v8::user_class::Derive) -> void {
      // Drop happens.
    }
    #[no_mangle]
    extern "C" fn OVERRIDE__v8__UserClass__DELETE(mem: Box<std::mem::MaybeUninit::<v8::user_class::Derive>>) -> void {
      // Drop happens.
    }
    #[no_mangle]
    extern "C" fn OVERRIDE__v8__UserClass__VirtualMethod(this: &mut v8::user_class::Derive, arg: i32) -> i32 {
      unsafe { this.vtable.VirtualMethod(std::mem::transmute(this), arg) }
    }
    #[no_mangle]
    extern "C" fn OVERRIDE__v8__UserClass__PureVirtualMethod(this: &v8::user_class::Derive) -> usize {
      unsafe { this.vtable.PureVirtualMethod(std::mem::transmute(this)) }
    }

    pub struct UserClass {
      base: UserClass,
    }

    pub trait Derive {
      // Non-pure virtual method has default impl.
      fn VirtualMethod(&mut self, arg: i32) -> i32 {
        v8__SomeClass__SomeClass__VirtualMethod(self, arg)
      }
      // Pure virtual method has no default impl.
      fn PureVirtualMethod(&self) -> usize;
    }
  }
}
```
