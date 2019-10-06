
#include <cassert>
#include <cstdint>
#include <cstdio>
#include <new>
#include <type_traits>
//#include <typeinfo>
#include <utility>

struct Target {
  int _m0;
  virtual void a() {}
  virtual void b() {}
  virtual void c() {}
  bool _m1;
};

struct Foo {
  int x;
  virtual int a() {
    return 1;
  }
  virtual int b() {
    return 1;
  }
  virtual int c() {
    return 1;
  }
  virtual int d() {
    return 1;
  }
  virtual int e() {
    return 1;
  }
  virtual int f() {
    return 1;
  }
};

template <int n>
struct dummy_t {
 private:
  virtual int _() {
    return n;
  }
};

template <int n, class U>
struct vt_seq_elem_t {
  using type = dummy_t<n>;
};

template <class U>
struct vt_seq_elem_t<0, U> {
  using type = U;
};

class vtable_impl_t {
  template <class>
  class vtable_base;
  template <class>
  class vtable_size;

 public:
  struct entry_t {
    const void* ptr;
  };

 protected:
  template <class U>
  static const entry_t* get_vtable_base(const U& obj) {
    return vtable_base<U>::get(obj);
  }

  template <class U>
  static const size_t get_vtable_size(const U& obj) {
    return vtable_size<U>::get(obj);
  }

 private:
  template <class A, class B>
  static constexpr void assert_same_layout() {
    static_assert(sizeof(A) == sizeof(B), "size mismatch");
    static_assert(alignof(A) == alignof(B), "alignment mismatch");
  }

  template <class U>
  class vtable_base {
    static_assert(std::is_polymorphic<U>::value, "not polymorphic");

    union overlay_t {
     private:
      const U obj_;
      const entry_t* vt_;

     public:
      overlay_t(const U& obj) : obj_(obj) {}
      ~overlay_t() {}
      const entry_t* vt() const {
        return vt_;
      }
    };

   public:
    static const entry_t* get(const U& obj) {
      const overlay_t& overlay = *reinterpret_cast<const overlay_t*>(&obj);
      return overlay.vt();
    }
  };

  template <class U>
  class vtable_size {
    static_assert(std::is_polymorphic<U>::value, "not polymorphic");

    struct vt_seq_t : vt_seq_elem_t<-2, U>::type,
                      vt_seq_elem_t<-1, U>::type,
                      U,
                      vt_seq_elem_t<1, U>::type,
                      vt_seq_elem_t<2, U>::type,
                      vt_seq_elem_t<3, U>::type {
      vt_seq_t(const U& obj) : U(obj) {}
    };

    template <int n>
    static const size_t vt_seq_size(const vt_seq_t& seq) {
      auto size =
          vtable_base<typename vt_seq_elem_t<n + 1, U>::type>::get(seq) -
          vtable_base<typename vt_seq_elem_t<n, U>::type>::get(seq);
      return static_cast<size_t>(size);
    }

   public:
    static size_t get(const U& obj) {
      vt_seq_t seq(obj);

      fprintf(stderr, "%d\n", (int) vt_seq_size<-2>(seq));
      fprintf(stderr, "%d\n", (int) vt_seq_size<-1>(seq));
      fprintf(stderr, "%d\n", (int) vt_seq_size<0>(seq));
      fprintf(stderr, "%d\n", (int) vt_seq_size<1>(seq));
      fprintf(stderr, "%d\n", (int) vt_seq_size<2>(seq));

      auto pre_boundary_vt_size = vt_seq_size<-1>(seq);
      auto post_boundary_vt_size = vt_seq_size<1>(seq);
      assert(pre_boundary_vt_size == vt_seq_size<1>(seq));
      assert(post_boundary_vt_size == vt_seq_size<-1>(seq));
      assert(pre_boundary_vt_size > 0);
      assert(post_boundary_vt_size > 0);

      /*
      auto pre_boundary_vt_size = vt_seq_size<-1>(seq);
      auto post_boundary_vt_size = vt_seq_size<1>(seq);
      assert(pre_boundary_vt_size == post_boundary_vt_size);
      assert(pre_boundary_vt_size > 0);
      assert(post_boundary_vt_size > 0);
      */

      auto inner_vt_size = vt_seq_size<0>(seq);
      // assert(inner_vt_size >= pre_boundary_vt_size);
      assert(inner_vt_size >= 0);

      return inner_vt_size;
    }
  };

  class assertions {
    class vtable_only_t {
      virtual void method() {}
    };

    template <class U>
    class data_only_t {
      U data_;

     public:
      data_only_t() : data_() {}
    };

    template <class U>
    class vtable_and_data_t {
      U data1_;
      virtual void method() {}
      U data2_;

     public:
      vtable_and_data_t() : data1_(), data2_() {}
    };

    template <class U>
    static void assert_has_vtable() {
      const U obj;
      assert(get_vtable_base(obj) != nullptr);
      assert(get_vtable_size(obj) > 0);
    }

   public:
    assertions() {
      assert_same_layout<vtable_only_t,
                         decltype(get_vtable_base(vtable_only_t()))>();
      assert_has_vtable<vtable_only_t>();
      assert_has_vtable<vtable_and_data_t<char>>();
      assert_has_vtable<vtable_and_data_t<int>>();
      assert_has_vtable<vtable_and_data_t<uintptr_t>>();
      assert_has_vtable<
          vtable_and_data_t<std::aligned_storage<sizeof(void*) * 3, 1>>>();
      assert_has_vtable<vtable_and_data_t<
          std::aligned_storage<sizeof(void*) * 3, alignof(void*)>>>();
    }
  };

  const entry_t* entries_;
  size_t size_;

 protected:
  vtable_impl_t(const entry_t* entries, size_t size)
      : entries_(entries), size_(size) {}
};

template <class T>
class vtable_t : public vtable_impl_t {
 public:
  explicit vtable_t(const T& obj = T())
      : vtable_impl_t(get_vtable_base(obj), get_vtable_size(obj)) {}
};


int main() {
  vtable_t<Target> v1;
  vtable_t<Foo> v2;
  return 0;
}
