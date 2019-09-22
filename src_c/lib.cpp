
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
};

class BB : public AA {
 public:
  BB() : AA(2){};

  void print(double d) {
    std::cout << "BB::print(" << d << ") -> ";
    AA::print(-1);
  }

  struct Rets {
    Rets(int a, int b, int c): n{a} {}
    void print() {
      std::cout << "Rets {" << std::endl;
      std::cout << "  nn: [ ";
      for (auto ni: n) {
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
    return Rets(a,b,c);
  }
  void print_rets(Rets& rets) {
    rets.print();
  }
};

// -- Wrapper --

// Instance methods.
template <class M, template <class, class, class...> class Functor>
class transform_method {
  template <class R, class T, class... A>
  static constexpr auto deduce(R (T::*)(A...))
      -> Functor<R, T, A...>;

  template <class R, class T, class... A>
  static constexpr auto deduce(R (T::*)(A...) const)
      -> Functor<R, const T, A...>;

 public:
  using type = decltype(deduce(std::declval<M>()));
};

template <class M, template <class, class, class...> class Functor>
using transform_method_t = typename transform_method<M, Functor>::type;

template <class T>
using wrap_type = T;

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

// Class instance methods
template <class M, M method>
class wrap_method_helper {
  template <class R, class T, class... A>
  struct make_wrapper;

  // Call method with return value.
  template <class R, class T, class... A>
  struct make_wrapper {
    static pod_t<R> invoke(wrap_type<T*> self, wrap_type<A>... args) {
      return pod<R>::into((self->*method)(args...));
    }
  };

  // Call method without return value.
  template <class T, class... A>
  struct make_wrapper<void, T, A...> {
    static void invoke(wrap_type<T*> self, wrap_type<A>... args) {
      (self->*method)(args...);
    }
  };

 public:
  static constexpr auto wrapper = transform_method_t<M, make_wrapper>::invoke;
};

#define wrap_method(method) \
  wrap_method_helper<decltype(method), method>::wrapper

extern "C" {
auto* AA_print = wrap_method(&AA::print);
auto* BB_print = wrap_method(&BB::print);
auto* BB_get_rets = wrap_method(&BB::get_rets);
auto* BB_print_rets = wrap_method(&BB::print_rets);
}