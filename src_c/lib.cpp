
#include <iostream>
#include <new>

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

// -- Wrapper --

template <class T>
struct pod {
  using type = std::aligned_storage_t<sizeof(T), alignof(T)>;

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

template <class T>
using pod_t = typename pod<T>::type;

// Helper class that deduces `this` type, return type, and argument types
// from a function prototype, and then applies functor that can then modify
// the function signature.
template <template <class, class, class...> class functor_template,
          class F,
          F fn>
class transform_function {
  template <class R, class... A>
  static constexpr auto select_functor(R (*)(A...))
      -> functor_template<void, R, A...>;

  template <class T, class R, class... A>
  static constexpr auto select_functor(R (T::*)(A...))
      -> functor_template<T, R, A...>;

  template <class T, class R, class... A>
  static constexpr auto select_functor(R (T::*)(A...) const)
      -> functor_template<const T, R, A...>;

  using functor = decltype(select_functor(fn));

 public:
  static constexpr auto result = functor::template result<fn>;
};

// In some ABIs the implicit "this" argument that is passed to non-static
// methods is passed in a special register. Since Rust doesn't support C++
// FFI, it doesn't know how to deal with this. This transformation wraps
// instance methods in ordinary functions that receive `this` as their first
// parameter.
template <class F, F fn>
class method_to_function {
  // Instance method.
  template <class T, class R, class... A>
  struct functor {
    template <F fn>
    static inline R result(T* self, A... args) {
      return (self->*fn)(args...);
    }
  };

  // Already-static method or ordinary function.
  template <class R, class... A>
  struct functor<void, R, A...> {
    template <F fn>
    static constexpr auto result = fn;
  };

 public:
  static constexpr auto result = transform_function<functor, F, fn>::result;
};

// Wraps a function that returns a non-POD object into a function that
// returns a POD object. This is necessary because some ABIs return small
// objects in registers when they're POD, while non-POD object are written to
// a caller-specified stack address. Since Rust only supports FFI with C, where
// all structs are POD by definition, it'll always return small structs on the
// stack.
template <class F, F fn>
class make_function_return_pod {
  template <class T, class R, class... A>
  struct functor;

  // Convert return value.
  template <class R, class... A>
  struct functor<void, R, A...> {
    template <F fn>
    static inline pod_t<R> result(A... args) {
      return pod<R>::into(fn(args...));
    }
  };

  // No return value.
  template <class... A>
  struct functor<void, void, A...> {
    template <F fn>
    static constexpr auto result = fn;
  };

 public:
  static constexpr auto result = transform_function<functor, F, fn>::result;
};

template <class F, F fn>
struct wrap_function_helper {
  static constexpr auto temp = method_to_function<decltype((fn)), fn>::result;
  static constexpr auto result =
      make_function_return_pod<decltype((temp)), temp>::result;
};

#define wrap_function(fn) wrap_function_helper<decltype(fn), fn>::result

extern "C" {
auto AA_print = wrap_function(&AA::print);
auto AA_powpow = wrap_function(&AA::powpow);
auto BB_print = wrap_function(&BB::print);
auto BB_get_rets = wrap_function(&BB::get_rets);
auto BB_print_rets = wrap_function(&BB::print_rets);
}
