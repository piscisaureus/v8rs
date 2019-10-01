
#include <unordered_map>
#include <typeinfo>
#include <memory>
#include <typeindex>
#include <optional>
#include <cassert>

class Prop {
friend class Context;
template <class P> 
const P& As() const {
  assert(std::type_index(typeid(*this)) == std::type_index(typeid(P)));
  return *reinterpret_cast<const P*>(this);
}
public:
  virtual ~Prop() = default;
};

class StringProp: public Prop {
  std::string value_;
protected:
  virtual const std::string label() const {
    return typeid(*this).name();
  }
public:
  using Value = std::string;

  StringProp(): value_() {};
  StringProp(const std::string& value): value_(value) {};
  StringProp(const char* value): value_(value) {};
  ~StringProp() {}

  const std::string& get() const {
    return value_;
  }
};

class Context {
  static thread_local Context* current_;

  Context* parent_;
  std::unordered_map<std::type_index, std::shared_ptr<Prop>> props;

  Context(Context&& that): parent_(Context::current_), props(std::move(that.props)) {
    if (parent_ == &that) {
      Context:current_ = this;
      parent_ = that.parent_;
      that.parent_ = nullptr;
    }
  }

  Context(): parent_(Context::current_) {
        std::cout << "push\n";
    if (parent_)
      props = parent_->props;
    Context:current_ = this;
  }

  template <class P>
  using Val = typename P::Value;

public:
  ~Context() {
    if (Context::current_ == this) {
       Context::current_ = parent_;
        std::cout << "pop\n";
    }
  }

  static const Context& current() {
    auto current = Context::current_;
    if (current == nullptr) 
      current = new Context();
    return *current;
  }

  template <class P>
  std::optional<Val<P>> try_get() const {
    auto key = std::type_index(typeid(P));
    if (props.count(key) > 0)
      return props.at(key)->As<P>().get();
    return {};
  }

  template <class P>
  const Val<P>& get() const {
    auto key = std::type_index(typeid(P));
        std::cout << "get " << key.hash_code() << "\n";
    if (props.count(key) > 0)
      return props.at(key)->As<P>().get();
    static const P default;
    return default.get();
  }

  template <class P>
  [[nodiscard]] Context set(const P& p) const {
    auto key = std::type_index(typeid(P));
    std::cout << "set " << key.hash_code() << "\n";
    Context c;
    c.props[key] = std::make_unique<P>(p.get());
    return c;
  }
};

thread_local Context* Context::current_ = nullptr;

class Namespace: public StringProp {
  using StringProp::StringProp;
  const std::string label() const override { return "NS"; }
};

class Henk: public StringProp {
  using StringProp::StringProp;
  const std::string label() const override { return "NS"; }
  public:
  Henk(): StringProp("I am henk") {};
};


int main() {
  std::cout << "H " << Context::current().get<Henk>() << "\n";
  std::cout << "N " << Context::current().get<Namespace>() << "\n";

  {
    Context::current().set(Namespace("Ja nu wel"));
    std::cout << "H " << Context::current().get<Henk>() << "\n";
    std::cout << "N " << Context::current().get<Namespace>() << "\n";
    {
      auto c3 = Context::current().set(Henk("Die nu ook"));
          std::cout << "H " << Context::current().get<Henk>() << "\n";
    std::cout << "N " << Context::current().get<Namespace>() << "\n";
    }
  }


  std::cout << "H " << Context::current().get<Henk>() << "\n";
  std::cout << "N " << Context::current().get<Namespace>() << "\n";

  return 1;
}