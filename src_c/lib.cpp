
#include <iostream>
#include <new>
#include <utility>

namespace {

class AA {
  int a_;

 public:
  AA(int a) : a_(a) {}

  void print(double d) {
    std::cout << "AA::print(" << d << ") " << a_ << std::endl;
  }

  virtual void virt1() {
    std::cout << "a";
  }
  virtual void virt2() {
    std::cout << "b";
  }

  static int powpow(int& a) {
    a *= a;
    return a * a;
  }

  static void staticx() {
    std::cout << "static";
  }
  void notvirt() {
    std::cout << "notvirt";
  }
};

class BB : public AA {
 public:
  BB() : AA(2){};

  void print(double d) {
    std::cout << "BB::print(" << d << ") -> ";
    AA::print(-1);
  }

  struct Rets {
    Rets(int a, int b, int c) : n{a} {}
    void print() {
      std::cout << "Rets {" << std::endl;
      std::cout << "  nn: [ ";
      for (auto ni : n) {
        std::cout << ni << ", ";
      }
      std::cout << "]" << std::endl;
      std::cout << "  b: " << b << std::endl;
      std::cout << "}" << std::endl;
    }

   private:
    int n[1];
    bool b = false;
  };

  Rets get_rets(int a, int b, int c) {
    return Rets(a, b, c);
  }
  void print_rets(Rets r1, Rets& r2) const {
    r1.print();
    r2.print();
  }

  virtual void virt1() {
    std::cout << "c";
  }
  virtual void virt2() {
    std::cout << "d";
  }
  virtual void virt3() {
    std::cout << "e";
  }
};

class CC {
  int* list;
public:
  explicit CC(int* l): list(l) {}
  int&& fifth() const {
    return std::move(list[5]);
  }
};

// -- Wrapper --
template <class T>
struct pod {
  using type =
      std::conditional_t<std::is_pod_v<T>,
                         T,
                         std::aligned_storage_t<sizeof(T), alignof(T)>>;

  static inline type into(T value) {
    // TODO: this violates aliasing rules pretty badly, but I don't see
    // a reasonable other way to achieve this.
    // Unfortunately we don't have std::launder until C++17.
    return *reinterpret_cast<type*>(&value);
  }
  static inline T from(type value) {
    return *reinterpret_cast<T*>(&value);
  }
};

// Helper class that deduces `this` type, return type, and argument types
// from a function prototype, and then applies functor that can then modify
// the function signature.
template <template <class, class, class...> class functor_template, class F>
class transform_function {
  template <class R, class... A>
  static constexpr auto select_functor(R (*)(A...))
      -> functor_template<void, R, A...>;

  template <class T, class R, class... A>
  static constexpr auto select_functor(R (T::*)(A...))
      -> functor_template<T&, R, A...>;

  template <class T, class R, class... A>
  static constexpr auto select_functor(R (T::*)(A...) const)
      -> functor_template<const T&, R, A...>;

  using functor = decltype(select_functor(std::declval<F>()));

 public:
  static constexpr auto result = functor::result;
};

// In some ABIs the implicit "this" argument that is passed to non-static
// methods is passed in a special register. Since Rust doesn't support C++
// FFI, it doesn't know how to deal with this. This transformation wraps
// instance methods in ordinary functions that receive `this` as their first
// parameter.
template <class F, F fn>
class make_static_method {
  // Instance method.
  template <class T, class R, class... A>
  struct functor {
    // template <F f>
    static inline R result(T self, A... args) {
      return (self.*fn)(std::forward<A>(args)...);
    }
  };

  // Already-static method or ordinary function.
  template <class R, class... A>
  struct functor<void, R, A...> {
    // template <F f>
    static constexpr auto result = fn;
  };

 public:
  static constexpr auto result = transform_function<functor, F>::result;
};

// Wraps a function that returns a non-POD object into a function that
// returns a POD object. This is necessary because some ABIs return small
// objects in registers when they're POD, while non-POD object are written to
// a caller-specified stack address. Since Rust only supports FFI with C, where
// all structs are POD by definition, it'll always return small structs on the
// stack.
template <class F, F fn>
class return_pod_to_rust {
  template <class T, class R, class... A>
  struct functor;

  // Convert by-value return value. Note that rvalue references are returned
  // by value as well.
  template <class R, class... A>
  struct functor<void, R, A...> {
    static inline typename pod<R>::type result(A... args) {
      return pod<R>::into(fn(std::forward<A>(args)...));
    }
  };

  // Preserve returned lvalue references.
  template <class R, class... A>
  struct functor<void, R&, A...> {
    static constexpr auto result = fn;
  };

  // No return value.
  template <class... A>
  struct functor<void, void, A...> {
    static constexpr auto result = fn;
  };

 public:
  static constexpr auto result = transform_function<functor, F>::result;
};

template <class F, F fn>
class wrap_function_impl {
  static constexpr auto f1 = make_static_method<decltype((fn)), fn>::result;
  static constexpr auto f2 = return_pod_to_rust<decltype((f1)), f1>::result;

 public:
  static constexpr std::add_const_t<decltype((f2))> result = f2;
};
template <auto fn>
static constexpr auto wrap_function =
    wrap_function_impl<decltype(fn), fn>::result;

template <class T, class... A>
struct wrap_class_new_impl {
  static T& call_new(A... args) {
    auto self = new T(std::forward<A>(args)...);
    return *self;
  }
  static T& call_constructor(T& addr, A... args) {
    new (reinterpret_cast<char*>(&addr)) T(std::forward<A>(args)...);
    return addr;
  }
  static constexpr auto result =
      std::make_pair(wrap_function<call_new>, wrap_function<call_constructor>);
  static_assert(sizeof(result) == sizeof(void (*)()) * 2);
};
template <class T, class... A>
static constexpr auto wrap_new = wrap_class_new_impl<T, A...>::result;

template <class T>
struct wrap_class_delete_impl {
  static void call_delete(T& self) {
    delete &self;
  }
  static void call_destructor(T& self) {
    self.~T();
  }
  static constexpr auto result = std::make_pair(
      wrap_function<call_delete>, wrap_function<call_destructor>);
  static_assert(sizeof(result) == sizeof(void (*)()) * 2);
};
template <class T>
static constexpr auto wrap_delete = wrap_class_delete_impl<T>::result;

}  // anonymous namespace

extern "C" {
auto AA_new = wrap_new<AA, int>;
auto AA_delete = wrap_delete<AA>;
auto AA_print = wrap_function<&AA::print>;
auto AA_powpow = wrap_function<&AA::powpow>;
auto BB_print = wrap_function<&BB::print>;
auto BB_get_rets = wrap_function<&BB::get_rets>;
auto BB_print_rets = wrap_function<&BB::print_rets>;
auto CC_new = wrap_new<CC, int*>;
auto CC_fifth = wrap_function<&CC::fifth>;
}
