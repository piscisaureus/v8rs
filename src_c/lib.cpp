
#include <iostream>
#include <memory>
#include <new>
#include <utility>

// -- Wrapper --
namespace {
template <class V>
using storage_t = std::aligned_storage_t<sizeof(V), alignof(V)>;

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
  using result_type = typename functor::result_type;

  template <F fn>
  static constexpr auto result = functor::template result<fn>;
  template <result_type fn>
  static constexpr auto imported = functor::template imported<fn>;
};

template <class F>
struct noop {
  using result_type = F;
  template <F fn>
  static constexpr F result = fn;
  template <F fn>
  static constexpr F imported = fn;
};

// In some ABIs the implicit "this" argument that is passed to non-static
// methods is passed in a special register. Since Rust doesn't support C++
// FFI, it doesn't know how to deal with this. This transformation wraps
// instance methods in ordinary functions that receive `this` as their first
// parameter.
template <class F>
class make_static_method {
  // Instance method.
  template <class T, class R, class... A>
  struct functor {
    using result_type = R (*)(T, A...);
    template <F fn>
    static inline R result(T self, A... args) {
      return (self.*fn)(std::forward<A>(args)...);
    }
    template <result_type fn>
    struct impl_class : public T {
      R invoke(A... args) override {
        return fn(*this, std::forward<A>(args)...);
      }
    };
    template <result_type fn>
    static constexpr F imported = &impl_class<fn>::invoke;
  };

  // Already-static method or ordinary function.
  template <class R, class... A>
  struct functor<void, R, A...> : noop<F> {};

 public:
  using result_type = typename transform_function<functor, F>::result_type;

  template <F fn>
  static constexpr auto result =
      transform_function<functor, F>::template result<fn>;
  template <result_type fn>
  static constexpr auto imported =
      transform_function<functor, F>::template imported<fn>;
};

// Wraps a function that returns a non-POD(*) object into a function that
// returns a POD object. This is necessary because some ABIs return small
// objects in registers when they're POD, while non-POD object are written to
// a caller-specified stack address. Since Rust only supports FFI with C, where
// all structs are POD by definition, it'll always return small structs on the
// stack.
// (*) MSFT uses the C++03 definition of POD, which is more strict than in
// later editions. Therefore we wrapp all class and union types.
template <class T>
struct nil_return_adapter {
  using abi_type = T;
  inline static T wrap(T val) {
    return val;
  };
  inline static T unwrap(T val) {
    return val;
  };
};

template <class T>
struct pod_return_adapter {
  using abi_type = std::conditional_t<std::is_const_v<T>,
                                      std::add_const_t<storage_t<T>>,
                                      storage_t<T>>;
  inline static abi_type& wrap(T&& val) {
    assert_equal_layout();
    return *std::launder(reinterpret_cast<abi_type*>(&val));
  }
  inline static T& unwrap(abi_type&& val) {
    assert_equal_layout();
    return *std::launder(reinterpret_cast<T*>(&val));
  }

 private:
  inline static void assert_equal_layout() {
    static_assert(std::is_pod_v<abi_type>, "not a POD type");
    static_assert(sizeof(abi_type) == sizeof(T), "size mismatch");
    static_assert(alignof(abi_type) == alignof(T), "alignment mismatch");
  }
};

template <class T>
using return_adapter =
    std::conditional_t<std::is_class_v<T> || std::is_union_v<T>,
                       pod_return_adapter<T>,
                       nil_return_adapter<T>>;

template <class F>
class return_pod_to_rust {
  template <class T, class R, class... A>
  struct functor;

  // Convert by-value return value. Note that rvalue references are returned
  // by value as well.
  template <class R, class... A>
  struct functor<void, R, A...> {
    using abi_return_type = typename return_adapter<R>::abi_type;
    using result_type = abi_return_type (*)(A...);
    template <F fn>
    static inline abi_return_type result(A... args) {
      return return_adapter<R>::wrap(fn(std::forward<A>(args)...));
    }
    template <result_type fn>
    static inline R imported(A... args) {
      return return_adapter<R>::unwrap(fn(std::forward<A>(args)...));
    }
  };

  // Preserve returned lvalue references.
  template <class R, class... A>
  struct functor<void, R&, A...> : noop<F> {};

  // No return value.
  template <class... A>
  struct functor<void, void, A...> : noop<F> {};

 public:
  using result_type = typename transform_function<functor, F>::result_type;

  template <F fn>
  static constexpr auto result =
      transform_function<functor, F>::template result<fn>;
  template <result_type fn>
  static constexpr auto imported =
      transform_function<functor, F>::template imported<fn>;
};

template <class F, F fn>
class wrap_function_impl {
  static constexpr auto f1 =
      make_static_method<decltype((fn))>::template result<fn>;
  static constexpr auto f2 =
      return_pod_to_rust<decltype((f1))>::template result<f1>;

 public:
  static constexpr std::add_const_t<decltype((f2))> result = f2;
};


template <class FnCxx>
class adapt_fn {
  using Fn0 = FnCxx;
  using Fn1 = typename make_static_method<Fn0>::result_type;
  using Fn2 = typename return_pod_to_rust<Fn1>::result_type;
  using FnAbi = Fn2;

  template <FnAbi f>
  struct import_impl {
    static constexpr Fn2 f2 = f;
    static constexpr Fn1 f1 = return_pod_to_rust<Fn1>::template imported<f2>;
    static constexpr Fn0 f0 = make_static_method<Fn0>::template imported<f1>;
  };

 public:
  using abi_type = FnAbi;

  template <FnAbi f>
  static constexpr std::add_const_t<FnCxx> imported = import_impl<f>::f0;
};

template <auto fn>
static constexpr auto wrap_function =
    wrap_function_impl<decltype(fn), fn>::result;

template <class T>
union wrap_class {
  template <class... An>
  void construct(An... an) {
    new (this) wrap_class(std::forward<An>(an)...);
  }

  // void construct(T&&) = delete;
  // void construct(T) = delete;
  // void construct(T) = delete;

  void destruct() {
    this->~wrap_class();
  }

 private:
  template <class... A>
  inline wrap_class(A... args) : value_(std::forward<A>(args)...) {}
  inline ~wrap_class() {}
  storage_t<T> storage_;
  T value_;
};
}  // anonymous namespace

namespace {

class AA {
  int a_;

 public:
  AA(int a, int b) : a_(a) {}

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
  BB() : AA(2, 0){};

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

  const Rets get_rets(int a, int b, int c) {
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
  explicit CC(int* l) : list(l) {}
  int& fifth() const {
    return list[5];
  }
};
}  // namespace

struct Foo {
  Foo(int aa = 3) : a(aa) {}
  void print() {
    std::cout << "Foo { a: " << a << " }\n";
  }

 private:
  int a;
};

void do_reverse_roles(int& a);


extern "C" {
// auto AA_xxx = wrap_function<&::new (std::declval<void*>) AA>;
// auto AA_xxx = wrap_xxx<AA, &std::allocator<AA>::allocate>();

auto AA_construct = wrap_function<&wrap_class<AA>::construct<int, int>>;
auto AA_destruct = wrap_function<&wrap_class<AA>::destruct>;
auto AA_print = wrap_function<&AA::print>;
auto AA_powpow = wrap_function<&AA::powpow>;
auto BB_print = wrap_function<&BB::print>;
auto BB_get_rets = wrap_function<&BB::get_rets>;
auto BB_print_rets = wrap_function<&BB::print_rets>;
auto CC_construct = wrap_function<&wrap_class<CC>::construct<int*>>;
auto CC_fifth = wrap_function<&CC::fifth>;
auto int_construct = wrap_function<&wrap_class<Foo>::construct<>>;
auto reverse_roles = wrap_function<&do_reverse_roles>;

extern storage_t<Foo> do_call_me_pls_rs(int& a);
}


auto wrap(int& a) {
  return do_call_me_pls_rs(a);
}

auto do_call_me_pls = adapt_fn<Foo (*)(int& a)>::template imported<wrap>;

void do_reverse_roles(int& a) {
  auto x = do_call_me_pls(a);
  x.print();
}