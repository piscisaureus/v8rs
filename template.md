
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
}

namespace DERIVE {
namespace v8 {
  class UserClass;
}
}

extern "C" {
  void DERIVE__v8__UserClass__DTOR()
  int DERIVE__v8__UserClass__VirtualMethod(
      ::DERIVE::v8::UserClass& self, int arg);
  size_t DERIVE__v8__UserClass__PureVirtualMethod(
      const ::DERIVE::v8::UserClass&);
}

namespace DERIVE {
namespace v8 {
  // The actual size of classes that are extended in rust is not known in C++,
  // rust derivate may add any number of data fields to it. Therefore they
  // should never be constructed in C++.

  class UserClass: final public ::v8::UserClass {
    // Prohibit construction from C++.
    UserClass() = delete;

  public:
    // TODO: constructor?

    virtual ~UserClass() {
      DERIVE__v8__UserClass__DTOR(*this);
    }

    int VirtualMethod(int arg) override {
      return DERIVE__v8__UserClass__VirtualMethod(arg);  
    }

    size_t PureVirtualMethod() const override {
      return DERIVE__v8__UserClass__PureVirtualMethod();
    }
  };
}
}
```

## Rust/C ABI wrapper

```rust
extern "C" {
  fn v8__SomeClass__CTOR(this: &mut std::mem::MaybeUninit::<v8::SomeClass> self, arg: i32) -> ();
  fn v8__SomeClass__DTOR(this: &mut SomeClass) -> ();

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

    pub struct UserClass([usize; 3]);

    impl UserClass {
      pub fn VirtualMethod(&mut self, arg: i32) -> i32 {
        v8__UserClass__VirtualMethod(self, arg)
      }
      pub fn PureVirtualMethod(&self) -> usize {
        v8__UserClass__PureVirtualMethod(self)
      }
    }

    pub struct Derive {
      
    }

    extern "C" fn DERIVE__v8__UserClass__VirtualMethod(this: &mut v8::UserClass, arg: i32) -> i32;
    extern "C" fn DERIVE__v8__UserClass__PureVirtualMethod(this: &v8::UserClass& self) -> usize;

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