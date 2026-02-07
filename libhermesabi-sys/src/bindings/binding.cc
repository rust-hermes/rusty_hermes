// Stable C bindings for Hermes JavaScript engine (rusty_v8 style).
// Flat extern "C" wrapper functions around the JSI C++ API.

#include "binding.hpp"

#include <hermes/hermes.h>
#include <jsi/jsi.h>

#include <cassert>
#include <cstring>
#include <memory>
#include <string>
#include <vector>

namespace jsi = facebook::jsi;

// ---------------------------------------------------------------------------
// Helpers to access protected Runtime members
// ---------------------------------------------------------------------------

// RuntimeAccessor exposes the protected static methods on jsi::Runtime
// so we can convert between PointerValue* and JSI types.
class RuntimeAccessor : public jsi::Runtime {
 public:
  // Expose the protected PointerValue type.
  using PV = jsi::Runtime::PointerValue;

  // Construct a JSI type T from a raw PointerValue*.
  // The PointerValue* is NOT cloned — caller must ensure it stays alive.
  template <typename T>
  static T make(PointerValue* pv) {
    return jsi::Runtime::make<T>(pv);
  }

  static PointerValue* getPointerValue(jsi::Pointer& p) {
    return jsi::Runtime::getPointerValue(p);
  }

  static const PointerValue* getPointerValue(const jsi::Pointer& p) {
    return jsi::Runtime::getPointerValue(p);
  }

  static const PointerValue* getPointerValue(const jsi::Value& v) {
    return jsi::Runtime::getPointerValue(v);
  }

  // Invalidate a PointerValue (release the handle).
  static void release(void* pv) {
    if (pv) {
      static_cast<PointerValue*>(pv)->invalidate();
    }
  }
};

// ---------------------------------------------------------------------------
// HermesRt: opaque runtime wrapper with exception state
// ---------------------------------------------------------------------------

struct HermesRt {
  std::unique_ptr<facebook::hermes::HermesRuntime> runtime;

  // Pending JS error value (heap-allocated jsi::Value, or nullptr).
  jsi::Value* pending_js_error = nullptr;

  // Pending native error message (strdup'd C string, or nullptr).
  char* pending_error_message = nullptr;

  void clearError() {
    delete pending_js_error;
    pending_js_error = nullptr;
    free(pending_error_message);
    pending_error_message = nullptr;
  }

  ~HermesRt() {
    clearError();
  }
};

// ---------------------------------------------------------------------------
// Conversion helpers between C HermesValue and C++ jsi::Value
// ---------------------------------------------------------------------------

// Extract a PointerValue* from a JSI Pointer-derived type and prevent
// the C++ destructor from invalidating it (we transfer ownership to C).
template <typename T>
static void* steal_pointer(T&& val) {
  RuntimeAccessor::PV* pv =
      RuntimeAccessor::getPointerValue(static_cast<jsi::Pointer&>(val));
  // Null out the C++ side so ~Pointer() won't call invalidate().
  // Pointer has a single member ptr_. We use the same trick as the rvalue
  // getters in jsi.h (e.g., Value::getString(Runtime&) &&).
  // Since Pointer layout is just { PointerValue* ptr_; }, we can null it:
  *reinterpret_cast<void**>(&val) = nullptr;
  return static_cast<void*>(pv);
}

// Convert a jsi::Value into a C HermesValue, transferring ownership of
// any PointerValue to the caller.
static HermesValue jsi_value_to_c(jsi::Value&& val) {
  HermesValue result;
  if (val.isUndefined()) {
    result.kind = HermesValueKind_Undefined;
    result.data.number = 0;
  } else if (val.isNull()) {
    result.kind = HermesValueKind_Null;
    result.data.number = 0;
  } else if (val.isBool()) {
    result.kind = HermesValueKind_Boolean;
    result.data.boolean = val.getBool();
  } else if (val.isNumber()) {
    result.kind = HermesValueKind_Number;
    result.data.number = val.getNumber();
  } else if (val.isSymbol()) {
    result.kind = HermesValueKind_Symbol;
    // Extract the PointerValue* from the Value's internal Pointer.
    const RuntimeAccessor::PV* pv = RuntimeAccessor::getPointerValue(val);
    result.data.pointer = const_cast<void*>(static_cast<const void*>(pv));
    // Null out the Value's pointer to prevent invalidation on destruction.
    // Value's internal data_.pointer.ptr_ is at the same offset as data_.pointer.
    // We need to reach into the Value and null its PointerValue*.
    // Since we can't easily reach in, we use a trick: move the value into
    // a temporary of the specific type, then steal from that.
    // Actually simpler: just memset the pointer region in the Value.
    // The Value is { kind_, Data { bool|double|Pointer } } where Pointer is { PointerValue* }.
    // After extracting the pointer, we null it so ~Value() is a no-op.
    // Value has kind_ (int) + padding + data_ (8 bytes). data_.pointer.ptr_ is at &data_.
    // We can't portably access this, so let's use a different approach:
    // move-construct into the specific type to steal ownership.
    // But Value doesn't let us do that either from the outside.
    //
    // Safest approach: clone the PV so the Value can safely destruct.
    // Actually, we CAN just reach in. Value layout: { ValueKind kind_; Data data_; }
    // Data is a union of { bool, double, Pointer }. Pointer is { PointerValue* ptr_ }.
    // So data_.pointer.ptr_ is at offset sizeof(ValueKind) within Value, padded to 8.
    // Let's just reach in and null the pointer:
    struct ValueLayout {
      int kind_;
      union {
        bool boolean;
        double number;
        void* pointer;
      } data_;
    };
    reinterpret_cast<ValueLayout*>(&val)->data_.pointer = nullptr;
  } else if (val.isBigInt()) {
    result.kind = HermesValueKind_BigInt;
    const RuntimeAccessor::PV* pv = RuntimeAccessor::getPointerValue(val);
    result.data.pointer = const_cast<void*>(static_cast<const void*>(pv));
    struct ValueLayout { int kind_; union { bool b; double n; void* p; } data_; };
    reinterpret_cast<ValueLayout*>(&val)->data_.p = nullptr;
  } else if (val.isString()) {
    result.kind = HermesValueKind_String;
    const RuntimeAccessor::PV* pv = RuntimeAccessor::getPointerValue(val);
    result.data.pointer = const_cast<void*>(static_cast<const void*>(pv));
    struct ValueLayout { int kind_; union { bool b; double n; void* p; } data_; };
    reinterpret_cast<ValueLayout*>(&val)->data_.p = nullptr;
  } else if (val.isObject()) {
    result.kind = HermesValueKind_Object;
    const RuntimeAccessor::PV* pv = RuntimeAccessor::getPointerValue(val);
    result.data.pointer = const_cast<void*>(static_cast<const void*>(pv));
    struct ValueLayout { int kind_; union { bool b; double n; void* p; } data_; };
    reinterpret_cast<ValueLayout*>(&val)->data_.p = nullptr;
  } else {
    result.kind = HermesValueKind_Undefined;
    result.data.number = 0;
  }
  return result;
}

// Reconstruct a jsi::Value from a C HermesValue. This CLONES the pointer
// (if present) so the C side retains ownership.
static jsi::Value c_to_jsi_value(jsi::Runtime& rt, const HermesValue* val) {
  switch (val->kind) {
    case HermesValueKind_Undefined:
      return jsi::Value::undefined();
    case HermesValueKind_Null:
      return jsi::Value(nullptr);
    case HermesValueKind_Boolean:
      return jsi::Value(val->data.boolean);
    case HermesValueKind_Number:
      return jsi::Value(val->data.number);
    case HermesValueKind_Symbol: {
      auto* pv = static_cast<RuntimeAccessor::PV*>(val->data.pointer);
      jsi::Symbol sym = RuntimeAccessor::make<jsi::Symbol>(pv);
      jsi::Value result(rt, sym);
      // Don't let sym's dtor invalidate the PV — the C side still owns it.
      *reinterpret_cast<void**>(&sym) = nullptr;
      return result;
    }
    case HermesValueKind_BigInt: {
      auto* pv = static_cast<RuntimeAccessor::PV*>(val->data.pointer);
      jsi::BigInt bi = RuntimeAccessor::make<jsi::BigInt>(pv);
      jsi::Value result(rt, bi);
      *reinterpret_cast<void**>(&bi) = nullptr;
      return result;
    }
    case HermesValueKind_String: {
      auto* pv = static_cast<RuntimeAccessor::PV*>(val->data.pointer);
      jsi::String str = RuntimeAccessor::make<jsi::String>(pv);
      jsi::Value result(rt, str);
      *reinterpret_cast<void**>(&str) = nullptr;
      return result;
    }
    case HermesValueKind_Object: {
      auto* pv = static_cast<RuntimeAccessor::PV*>(val->data.pointer);
      jsi::Object obj = RuntimeAccessor::make<jsi::Object>(pv);
      jsi::Value result(rt, obj);
      *reinterpret_cast<void**>(&obj) = nullptr;
      return result;
    }
  }
  return jsi::Value::undefined();
}

// Helper: make an undefined HermesValue (used as error-return sentinel).
static HermesValue make_undefined() {
  HermesValue v;
  v.kind = HermesValueKind_Undefined;
  v.data.number = 0;
  return v;
}

// Macros for try/catch wrapping.
#define HERMES_TRY(rt) try {
#define HERMES_CATCH_VALUE(rt) \
  } catch (const jsi::JSError& e) { \
    (rt)->clearError(); \
    (rt)->pending_js_error = new jsi::Value(*(rt)->runtime, e.value()); \
    return make_undefined(); \
  } catch (const std::exception& e) { \
    (rt)->clearError(); \
    (rt)->pending_error_message = strdup(e.what()); \
    return make_undefined(); \
  }

#define HERMES_CATCH_PTR(rt) \
  } catch (const jsi::JSError& e) { \
    (rt)->clearError(); \
    (rt)->pending_js_error = new jsi::Value(*(rt)->runtime, e.value()); \
    return nullptr; \
  } catch (const std::exception& e) { \
    (rt)->clearError(); \
    (rt)->pending_error_message = strdup(e.what()); \
    return nullptr; \
  }

#define HERMES_CATCH_BOOL(rt) \
  } catch (const jsi::JSError& e) { \
    (rt)->clearError(); \
    (rt)->pending_js_error = new jsi::Value(*(rt)->runtime, e.value()); \
    return false; \
  } catch (const std::exception& e) { \
    (rt)->clearError(); \
    (rt)->pending_error_message = strdup(e.what()); \
    return false; \
  }

#define HERMES_CATCH_VOID(rt) \
  } catch (const jsi::JSError& e) { \
    (rt)->clearError(); \
    (rt)->pending_js_error = new jsi::Value(*(rt)->runtime, e.value()); \
    return; \
  } catch (const std::exception& e) { \
    (rt)->clearError(); \
    (rt)->pending_error_message = strdup(e.what()); \
    return; \
  }

// Get the jsi::Runtime& from HermesRt.
static inline jsi::Runtime& rt(HermesRt* hrt) {
  return *hrt->runtime;
}

// Reconstruct a JSI Pointer type from an opaque void* without cloning.
// The returned object does NOT own the PointerValue — the caller must
// null it out before it goes out of scope.
template <typename T>
static T borrow(const void* pv) {
  return RuntimeAccessor::make<T>(
      static_cast<RuntimeAccessor::PV*>(const_cast<void*>(pv)));
}

// RAII guard that nulls out a Pointer's internal ptr_ on destruction,
// preventing invalidate() from being called.
template <typename T>
class Borrowed {
 public:
  explicit Borrowed(const void* pv) : val_(borrow<T>(pv)) {}
  ~Borrowed() { *reinterpret_cast<void**>(&val_) = nullptr; }
  T& get() { return val_; }
  const T& get() const { return val_; }
 private:
  T val_;
};

// ===========================================================================
// extern "C" implementations
// ===========================================================================

extern "C" {

// ---------------------------------------------------------------------------
// Runtime lifecycle
// ---------------------------------------------------------------------------

HermesRt* hermes__Runtime__New(void) {
  auto hrt = new HermesRt();
  hrt->runtime = facebook::hermes::makeHermesRuntime();
  return hrt;
}

void hermes__Runtime__Delete(HermesRt* hrt) {
  delete hrt;
}

bool hermes__Runtime__HasPendingError(const HermesRt* hrt) {
  return hrt->pending_js_error != nullptr ||
         hrt->pending_error_message != nullptr;
}

struct HermesValue hermes__Runtime__GetAndClearError(HermesRt* hrt) {
  if (hrt->pending_js_error) {
    HermesValue result = jsi_value_to_c(std::move(*hrt->pending_js_error));
    delete hrt->pending_js_error;
    hrt->pending_js_error = nullptr;
    return result;
  }
  return make_undefined();
}

const char* hermes__Runtime__GetAndClearErrorMessage(HermesRt* hrt) {
  const char* msg = hrt->pending_error_message;
  hrt->pending_error_message = nullptr;
  return msg; // Caller must free() this.
}

void* hermes__Runtime__Global(HermesRt* hrt) {
  jsi::Object global = hrt->runtime->global();
  return steal_pointer(std::move(global));
}

// ---------------------------------------------------------------------------
// Evaluate
// ---------------------------------------------------------------------------

struct HermesValue hermes__Runtime__EvaluateJavaScript(
    HermesRt* hrt,
    const uint8_t* data,
    size_t len,
    const char* source_url,
    size_t source_url_len) {
  HERMES_TRY(hrt)
  std::string url(source_url, source_url_len);
  auto buf = std::make_shared<jsi::StringBuffer>(std::string(
      reinterpret_cast<const char*>(data), len));
  jsi::Value result = hrt->runtime->evaluateJavaScript(buf, url);
  return jsi_value_to_c(std::move(result));
  HERMES_CATCH_VALUE(hrt)
}

int hermes__Runtime__DrainMicrotasks(HermesRt* hrt, int max_hint) {
  HERMES_TRY(hrt)
  bool drained = hrt->runtime->drainMicrotasks(max_hint);
  return drained ? 1 : 0;
  } catch (const jsi::JSError& e) {
    hrt->clearError();
    hrt->pending_js_error = new jsi::Value(*hrt->runtime, e.value());
    return -1;
  } catch (const std::exception& e) {
    hrt->clearError();
    hrt->pending_error_message = strdup(e.what());
    return -1;
  }
}

// ---------------------------------------------------------------------------
// String
// ---------------------------------------------------------------------------

void* hermes__String__CreateFromUtf8(
    HermesRt* hrt,
    const uint8_t* utf8,
    size_t len) {
  HERMES_TRY(hrt)
  jsi::String str = jsi::String::createFromUtf8(rt(hrt), utf8, len);
  return steal_pointer(std::move(str));
  HERMES_CATCH_PTR(hrt)
}

void* hermes__String__CreateFromAscii(
    HermesRt* hrt,
    const char* ascii,
    size_t len) {
  HERMES_TRY(hrt)
  jsi::String str = jsi::String::createFromAscii(rt(hrt), ascii, len);
  return steal_pointer(std::move(str));
  HERMES_CATCH_PTR(hrt)
}

size_t hermes__String__ToUtf8(
    HermesRt* hrt,
    const void* str,
    char* buf,
    size_t buf_len) {
  Borrowed<jsi::String> s(str);
  std::string utf8 = s.get().utf8(rt(hrt));
  size_t needed = utf8.size();
  if (buf && buf_len > 0) {
    size_t to_copy = needed < buf_len ? needed : buf_len;
    memcpy(buf, utf8.data(), to_copy);
  }
  return needed;
}

bool hermes__String__StrictEquals(
    HermesRt* hrt,
    const void* a,
    const void* b) {
  Borrowed<jsi::String> sa(a);
  Borrowed<jsi::String> sb(b);
  return jsi::String::strictEquals(rt(hrt), sa.get(), sb.get());
}

void hermes__String__Release(void* pv) {
  if (pv) {
    RuntimeAccessor::release(pv);
  }
}

// ---------------------------------------------------------------------------
// PropNameID
// ---------------------------------------------------------------------------

void* hermes__PropNameID__ForAscii(HermesRt* hrt, const char* str, size_t len) {
  HERMES_TRY(hrt)
  jsi::PropNameID pni = jsi::PropNameID::forAscii(rt(hrt), str, len);
  return steal_pointer(std::move(pni));
  HERMES_CATCH_PTR(hrt)
}

void* hermes__PropNameID__ForUtf8(
    HermesRt* hrt,
    const uint8_t* utf8,
    size_t len) {
  HERMES_TRY(hrt)
  jsi::PropNameID pni = jsi::PropNameID::forUtf8(rt(hrt), utf8, len);
  return steal_pointer(std::move(pni));
  HERMES_CATCH_PTR(hrt)
}

void* hermes__PropNameID__ForString(HermesRt* hrt, const void* str) {
  HERMES_TRY(hrt)
  Borrowed<jsi::String> s(str);
  jsi::PropNameID pni = jsi::PropNameID::forString(rt(hrt), s.get());
  return steal_pointer(std::move(pni));
  HERMES_CATCH_PTR(hrt)
}

size_t hermes__PropNameID__ToUtf8(
    HermesRt* hrt,
    const void* pni,
    char* buf,
    size_t buf_len) {
  Borrowed<jsi::PropNameID> p(pni);
  std::string utf8 = p.get().utf8(rt(hrt));
  size_t needed = utf8.size();
  if (buf && buf_len > 0) {
    size_t to_copy = needed < buf_len ? needed : buf_len;
    memcpy(buf, utf8.data(), to_copy);
  }
  return needed;
}

bool hermes__PropNameID__Equals(
    HermesRt* hrt,
    const void* a,
    const void* b) {
  Borrowed<jsi::PropNameID> pa(a);
  Borrowed<jsi::PropNameID> pb(b);
  return jsi::PropNameID::compare(rt(hrt), pa.get(), pb.get());
}

void hermes__PropNameID__Release(void* pv) {
  if (pv) {
    RuntimeAccessor::release(pv);
  }
}

// ---------------------------------------------------------------------------
// Object
// ---------------------------------------------------------------------------

void* hermes__Object__New(HermesRt* hrt) {
  jsi::Object obj(rt(hrt));
  return steal_pointer(std::move(obj));
}

struct HermesValue hermes__Object__GetProperty__String(
    HermesRt* hrt,
    const void* obj,
    const void* name) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  Borrowed<jsi::String> n(name);
  jsi::Value result = o.get().getProperty(rt(hrt), n.get());
  return jsi_value_to_c(std::move(result));
  HERMES_CATCH_VALUE(hrt)
}

struct HermesValue hermes__Object__GetProperty__PropNameID(
    HermesRt* hrt,
    const void* obj,
    const void* name) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  Borrowed<jsi::PropNameID> n(name);
  jsi::Value result = o.get().getProperty(rt(hrt), n.get());
  return jsi_value_to_c(std::move(result));
  HERMES_CATCH_VALUE(hrt)
}

bool hermes__Object__SetProperty__String(
    HermesRt* hrt,
    const void* obj,
    const void* name,
    const struct HermesValue* val) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  Borrowed<jsi::String> n(name);
  jsi::Value v = c_to_jsi_value(rt(hrt), val);
  o.get().setProperty(rt(hrt), n.get(), std::move(v));
  return true;
  HERMES_CATCH_BOOL(hrt)
}

bool hermes__Object__SetProperty__PropNameID(
    HermesRt* hrt,
    const void* obj,
    const void* name,
    const struct HermesValue* val) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  Borrowed<jsi::PropNameID> n(name);
  jsi::Value v = c_to_jsi_value(rt(hrt), val);
  o.get().setProperty(rt(hrt), n.get(), std::move(v));
  return true;
  HERMES_CATCH_BOOL(hrt)
}

bool hermes__Object__HasProperty__String(
    HermesRt* hrt,
    const void* obj,
    const void* name) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  Borrowed<jsi::String> n(name);
  return o.get().hasProperty(rt(hrt), n.get());
  HERMES_CATCH_BOOL(hrt)
}

bool hermes__Object__HasProperty__PropNameID(
    HermesRt* hrt,
    const void* obj,
    const void* name) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  Borrowed<jsi::PropNameID> n(name);
  return o.get().hasProperty(rt(hrt), n.get());
  HERMES_CATCH_BOOL(hrt)
}

void* hermes__Object__GetPropertyNames(HermesRt* hrt, const void* obj) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  jsi::Array names = o.get().getPropertyNames(rt(hrt));
  return steal_pointer(std::move(names));
  HERMES_CATCH_PTR(hrt)
}

bool hermes__Object__IsArray(HermesRt* hrt, const void* obj) {
  Borrowed<jsi::Object> o(obj);
  return o.get().isArray(rt(hrt));
}

bool hermes__Object__IsFunction(HermesRt* hrt, const void* obj) {
  Borrowed<jsi::Object> o(obj);
  return o.get().isFunction(rt(hrt));
}

bool hermes__Object__IsArrayBuffer(HermesRt* hrt, const void* obj) {
  Borrowed<jsi::Object> o(obj);
  return o.get().isArrayBuffer(rt(hrt));
}

bool hermes__Object__StrictEquals(
    HermesRt* hrt,
    const void* a,
    const void* b) {
  Borrowed<jsi::Object> oa(a);
  Borrowed<jsi::Object> ob(b);
  return jsi::Object::strictEquals(rt(hrt), oa.get(), ob.get());
}

bool hermes__Object__InstanceOf(
    HermesRt* hrt,
    const void* obj,
    const void* func) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  Borrowed<jsi::Function> f(func);
  return o.get().instanceOf(rt(hrt), f.get());
  HERMES_CATCH_BOOL(hrt)
}

void hermes__Object__Release(void* pv) {
  if (pv) {
    RuntimeAccessor::release(pv);
  }
}

// ---------------------------------------------------------------------------
// Array
// ---------------------------------------------------------------------------

void* hermes__Array__New(HermesRt* hrt, size_t length) {
  HERMES_TRY(hrt)
  jsi::Array arr(rt(hrt), length);
  return steal_pointer(std::move(arr));
  HERMES_CATCH_PTR(hrt)
}

size_t hermes__Array__Size(HermesRt* hrt, const void* arr) {
  Borrowed<jsi::Array> a(arr);
  return a.get().size(rt(hrt));
}

struct HermesValue hermes__Array__GetValueAtIndex(
    HermesRt* hrt,
    const void* arr,
    size_t index) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Array> a(arr);
  jsi::Value result = a.get().getValueAtIndex(rt(hrt), index);
  return jsi_value_to_c(std::move(result));
  HERMES_CATCH_VALUE(hrt)
}

bool hermes__Array__SetValueAtIndex(
    HermesRt* hrt,
    const void* arr,
    size_t index,
    const struct HermesValue* val) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Array> a(arr);
  jsi::Value v = c_to_jsi_value(rt(hrt), val);
  a.get().setValueAtIndex(rt(hrt), index, std::move(v));
  return true;
  HERMES_CATCH_BOOL(hrt)
}

void hermes__Array__Release(void* pv) {
  if (pv) {
    RuntimeAccessor::release(pv);
  }
}

// ---------------------------------------------------------------------------
// Function
// ---------------------------------------------------------------------------

// Internal wrapper that bridges C callback to jsi::HostFunctionType.
struct HostFunctionClosure {
  HermesRt* hrt;
  HermesHostFunctionCallback callback;
  void* user_data;
  HermesHostFunctionFinalizer finalizer;

  ~HostFunctionClosure() {
    if (finalizer && user_data) {
      finalizer(user_data);
    }
  }
};

struct HermesValue hermes__Function__Call(
    HermesRt* hrt,
    const void* func,
    const struct HermesValue* this_val,
    const struct HermesValue* args,
    size_t argc) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Function> f(func);

  // Convert args from C to jsi::Value.
  std::vector<jsi::Value> jsi_args;
  jsi_args.reserve(argc);
  for (size_t i = 0; i < argc; ++i) {
    jsi_args.push_back(c_to_jsi_value(rt(hrt), &args[i]));
  }

  const jsi::Value* args_data = jsi_args.data();
  size_t args_count = jsi_args.size();

  jsi::Value result;
  if (this_val && this_val->kind == HermesValueKind_Object) {
    // callWithThis
    Borrowed<jsi::Object> thisObj(this_val->data.pointer);
    result = f.get().callWithThis(
        rt(hrt), thisObj.get(), args_data, args_count);
  } else {
    result = f.get().call(rt(hrt), args_data, args_count);
  }
  return jsi_value_to_c(std::move(result));
  HERMES_CATCH_VALUE(hrt)
}

struct HermesValue hermes__Function__CallAsConstructor(
    HermesRt* hrt,
    const void* func,
    const struct HermesValue* args,
    size_t argc) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Function> f(func);

  std::vector<jsi::Value> jsi_args;
  jsi_args.reserve(argc);
  for (size_t i = 0; i < argc; ++i) {
    jsi_args.push_back(c_to_jsi_value(rt(hrt), &args[i]));
  }

  const jsi::Value* args_data = jsi_args.data();
  size_t args_count = jsi_args.size();
  jsi::Value result =
      f.get().callAsConstructor(rt(hrt), args_data, args_count);
  return jsi_value_to_c(std::move(result));
  HERMES_CATCH_VALUE(hrt)
}

void* hermes__Function__CreateFromHostFunction(
    HermesRt* hrt,
    const void* name,
    unsigned int param_count,
    HermesHostFunctionCallback callback,
    void* user_data,
    HermesHostFunctionFinalizer finalizer) {
  HERMES_TRY(hrt)
  auto closure = std::make_shared<HostFunctionClosure>();
  closure->hrt = hrt;
  closure->callback = callback;
  closure->user_data = user_data;
  closure->finalizer = finalizer;

  Borrowed<jsi::PropNameID> pni(name);

  jsi::Function func = jsi::Function::createFromHostFunction(
      rt(hrt),
      pni.get(),
      param_count,
      [closure](
          jsi::Runtime& /*runtime*/,
          const jsi::Value& thisVal,
          const jsi::Value* args,
          size_t count) -> jsi::Value {
        // Convert thisVal to C.
        // We need to borrow (not steal) the thisVal and args since they're
        // owned by the caller.
        HermesValue c_this;
        if (thisVal.isObject()) {
          c_this.kind = HermesValueKind_Object;
          c_this.data.pointer = const_cast<void*>(static_cast<const void*>(
              RuntimeAccessor::getPointerValue(thisVal)));
        } else if (thisVal.isUndefined()) {
          c_this.kind = HermesValueKind_Undefined;
          c_this.data.number = 0;
        } else {
          c_this.kind = HermesValueKind_Undefined;
          c_this.data.number = 0;
        }

        // Convert args to C (borrowing — no ownership transfer).
        std::vector<HermesValue> c_args(count);
        for (size_t i = 0; i < count; ++i) {
          const jsi::Value& arg = args[i];
          if (arg.isUndefined()) {
            c_args[i].kind = HermesValueKind_Undefined;
            c_args[i].data.number = 0;
          } else if (arg.isNull()) {
            c_args[i].kind = HermesValueKind_Null;
            c_args[i].data.number = 0;
          } else if (arg.isBool()) {
            c_args[i].kind = HermesValueKind_Boolean;
            c_args[i].data.boolean = arg.getBool();
          } else if (arg.isNumber()) {
            c_args[i].kind = HermesValueKind_Number;
            c_args[i].data.number = arg.getNumber();
          } else if (arg.isString()) {
            c_args[i].kind = HermesValueKind_String;
            c_args[i].data.pointer = const_cast<void*>(
                static_cast<const void*>(
                    RuntimeAccessor::getPointerValue(arg)));
          } else if (arg.isObject()) {
            c_args[i].kind = HermesValueKind_Object;
            c_args[i].data.pointer = const_cast<void*>(
                static_cast<const void*>(
                    RuntimeAccessor::getPointerValue(arg)));
          } else if (arg.isSymbol()) {
            c_args[i].kind = HermesValueKind_Symbol;
            c_args[i].data.pointer = const_cast<void*>(
                static_cast<const void*>(
                    RuntimeAccessor::getPointerValue(arg)));
          } else if (arg.isBigInt()) {
            c_args[i].kind = HermesValueKind_BigInt;
            c_args[i].data.pointer = const_cast<void*>(
                static_cast<const void*>(
                    RuntimeAccessor::getPointerValue(arg)));
          } else {
            c_args[i].kind = HermesValueKind_Undefined;
            c_args[i].data.number = 0;
          }
        }

        HermesValue c_result = closure->callback(
            closure->hrt,
            &c_this,
            c_args.data(),
            count,
            closure->user_data);

        // Convert result back to jsi::Value. The C callback returns an
        // OWNED HermesValue — we transfer ownership into the jsi::Value.
        // For pointer kinds, we reconstruct a JSI type that takes ownership.
        switch (c_result.kind) {
          case HermesValueKind_Undefined:
            return jsi::Value::undefined();
          case HermesValueKind_Null:
            return jsi::Value(nullptr);
          case HermesValueKind_Boolean:
            return jsi::Value(c_result.data.boolean);
          case HermesValueKind_Number:
            return jsi::Value(c_result.data.number);
          case HermesValueKind_Symbol:
            return jsi::Value(RuntimeAccessor::make<jsi::Symbol>(
                static_cast<RuntimeAccessor::PV*>(
                    c_result.data.pointer)));
          case HermesValueKind_BigInt:
            return jsi::Value(RuntimeAccessor::make<jsi::BigInt>(
                static_cast<RuntimeAccessor::PV*>(
                    c_result.data.pointer)));
          case HermesValueKind_String:
            return jsi::Value(RuntimeAccessor::make<jsi::String>(
                static_cast<RuntimeAccessor::PV*>(
                    c_result.data.pointer)));
          case HermesValueKind_Object:
            return jsi::Value(RuntimeAccessor::make<jsi::Object>(
                static_cast<RuntimeAccessor::PV*>(
                    c_result.data.pointer)));
        }
        return jsi::Value::undefined();
      });

  return steal_pointer(std::move(func));
  HERMES_CATCH_PTR(hrt)
}

bool hermes__Function__IsHostFunction(HermesRt* hrt, const void* func) {
  Borrowed<jsi::Function> f(func);
  return f.get().isHostFunction(rt(hrt));
}

void hermes__Function__Release(void* pv) {
  if (pv) {
    RuntimeAccessor::release(pv);
  }
}

// ---------------------------------------------------------------------------
// Value
// ---------------------------------------------------------------------------

void hermes__Value__Release(struct HermesValue* val) {
  if (val && val->kind >= HermesValueKind_Symbol && val->data.pointer) {
    RuntimeAccessor::release(val->data.pointer);
    val->data.pointer = nullptr;
  }
}

bool hermes__Value__StrictEquals(
    HermesRt* hrt,
    const struct HermesValue* a,
    const struct HermesValue* b) {
  jsi::Value va = c_to_jsi_value(rt(hrt), a);
  jsi::Value vb = c_to_jsi_value(rt(hrt), b);
  bool result = jsi::Value::strictEquals(rt(hrt), va, vb);
  // The c_to_jsi_value cloned the pointer values, so they'll be properly
  // cleaned up when va/vb go out of scope.
  return result;
}

// ---------------------------------------------------------------------------
// Symbol
// ---------------------------------------------------------------------------

void* hermes__Symbol__ToString(HermesRt* hrt, const void* sym) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Symbol> s(sym);
  std::string str = s.get().toString(rt(hrt));
  jsi::String js_str = jsi::String::createFromUtf8(
      rt(hrt), reinterpret_cast<const uint8_t*>(str.data()), str.size());
  return steal_pointer(std::move(js_str));
  HERMES_CATCH_PTR(hrt)
}

bool hermes__Symbol__StrictEquals(
    HermesRt* hrt,
    const void* a,
    const void* b) {
  Borrowed<jsi::Symbol> sa(a);
  Borrowed<jsi::Symbol> sb(b);
  return jsi::Symbol::strictEquals(rt(hrt), sa.get(), sb.get());
}

void hermes__Symbol__Release(void* pv) {
  if (pv) {
    RuntimeAccessor::release(pv);
  }
}

// ---------------------------------------------------------------------------
// BigInt
// ---------------------------------------------------------------------------

void* hermes__BigInt__FromInt64(HermesRt* hrt, int64_t val) {
  HERMES_TRY(hrt)
  jsi::BigInt bi = jsi::BigInt::fromInt64(rt(hrt), val);
  return steal_pointer(std::move(bi));
  HERMES_CATCH_PTR(hrt)
}

void* hermes__BigInt__FromUint64(HermesRt* hrt, uint64_t val) {
  HERMES_TRY(hrt)
  jsi::BigInt bi = jsi::BigInt::fromUint64(rt(hrt), val);
  return steal_pointer(std::move(bi));
  HERMES_CATCH_PTR(hrt)
}

bool hermes__BigInt__IsInt64(HermesRt* hrt, const void* bi) {
  Borrowed<jsi::BigInt> b(bi);
  return b.get().isInt64(rt(hrt));
}

bool hermes__BigInt__IsUint64(HermesRt* hrt, const void* bi) {
  Borrowed<jsi::BigInt> b(bi);
  return b.get().isUint64(rt(hrt));
}

uint64_t hermes__BigInt__Truncate(HermesRt* hrt, const void* bi) {
  Borrowed<jsi::BigInt> b(bi);
  return b.get().getUint64(rt(hrt));
}

void hermes__BigInt__Release(void* pv) {
  if (pv) {
    RuntimeAccessor::release(pv);
  }
}

// ---------------------------------------------------------------------------
// WeakObject
// ---------------------------------------------------------------------------

void* hermes__WeakObject__Create(HermesRt* hrt, const void* obj) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  jsi::WeakObject wo(rt(hrt), o.get());
  return steal_pointer(std::move(wo));
  HERMES_CATCH_PTR(hrt)
}

struct HermesValue hermes__WeakObject__Lock(HermesRt* hrt, const void* wo) {
  HERMES_TRY(hrt)
  Borrowed<jsi::WeakObject> w(wo);
  jsi::Value result = w.get().lock(rt(hrt));
  return jsi_value_to_c(std::move(result));
  HERMES_CATCH_VALUE(hrt)
}

void hermes__WeakObject__Release(void* pv) {
  if (pv) {
    RuntimeAccessor::release(pv);
  }
}

// ---------------------------------------------------------------------------
// HermesRuntime-specific (static)
// ---------------------------------------------------------------------------

bool hermes__IsHermesBytecode(const uint8_t* data, size_t len) {
  return facebook::hermes::HermesRuntime::isHermesBytecode(data, len);
}

uint32_t hermes__GetBytecodeVersion(void) {
  return facebook::hermes::HermesRuntime::getBytecodeVersion();
}

} // extern "C"
