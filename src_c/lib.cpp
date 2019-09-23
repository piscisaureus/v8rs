
#include <iostream>
#include <new>

class AA {
  int a_;

 public:
  AA(int a) : a_(a) {}

  void print(double d) {
    std::cout << "AA::print(" << d << ") " << a_ << std::endl;
  }

  virtual void virt() {}

  static int i_am_static(void* k) { return 3; }
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
};

// -- Wrapper --

template <class T>
struct pod {
  using type = std::aligned_storage_t<sizeof(T), alignof(T)>;

  static type into(T value) {
    // TODO: this violates aliasing rules pretty badly, but I don't see
    // a reasonable other way to achieve this.
    // Unfortunately we don't have std::launder until C++17.
    return *reinterpret_cast<type*>(&value);
  }
  static T from(type value) {
    return *reinterpret_cast<T*>(&value);
  }
};

template <class T>
using pod_t = typename pod<T>::type;

// Functions and methods
template <class F, template <class, class, class...> class Functor>
class transform_function_helper {
  template <class R, class... A>
  static constexpr auto deduce(R (*)(A...)) -> Functor<void, R, A...>;

  template <class T, class R, class... A>
  static constexpr auto deduce(R (T::*)(A...)) -> Functor<T, R, A...>;

  template <class T, class R, class... A>
  static constexpr auto deduce(R (T::*)(A...) const)
      -> Functor<const T, R, A...>;

 public:
  using result = decltype(deduce(std::declval<F>()));
};

template <class F, template <class, class, class...> class Functor>
using transform_function = typename transform_function_helper<F, Functor>::result;

// Convert methods to ordinary functions with `this` as the first argument.
template <class F>
class method_to_function_helper {
  // Instance method.
  template <class T, class R, class... A>
  struct transform {
    template<F fn>
    static R result(T* self, A... args) {
      return (self->*fn)(args...);
    }
  };

  // Already-static method or ordinary function.
  template <class R, class... A>
  struct transform<void, R, A...> {
    template<F fn>
    static constexpr auto result = fn;
  };

 public:
  template <F fn>
  static constexpr auto result = transform_function<F, transform>::template result<fn>;
};

template <class F, F fn>
static constexpr auto method_to_function = method_to_function_helper<F>::template result<fn>;

// Class instance methods
template <class F>
class function_return_pod_helper {
  template <class T, class R, class... A>
  struct transform;

  // Call method with return value.
  template <class R, class... A>
  struct transform<void, R, A...> {
    template<F fn>
    static pod_t<R> result(A... args) {
      return pod<R>::into(fn(args...));
    }
  };

  // No return value.
  template <class... A>
  struct transform<void, void, A...> {
    template<F fn>
    static constexpr auto result = fn;
  };

 public:
  template <F fn>
  static constexpr auto result = transform_function<F, transform>::template result<fn>;
};

template <class F, F fn>
static constexpr auto function_return_pod = function_return_pod_helper<F>::template result<fn>;

#define wrap_method_1(method) \
  method_to_function<decltype(method), method>
#define wrap_method(method) \
  function_return_pod<decltype((wrap_method_1(method))), (wrap_method_1(method))>

extern "C" {
auto* AA_print = wrap_method(&AA::print);
auto* BB_print = wrap_method(&BB::print);
auto* BB_get_rets = wrap_method(&BB::get_rets);
auto* BB_print_rets = wrap_method(&BB::print_rets);
auto* AA_i_am_static = wrap_method(&AA::i_am_static);
}