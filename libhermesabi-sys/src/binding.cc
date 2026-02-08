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
  // Null the Pointer's internal ptr_ so ~Pointer() won't call invalidate().
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
    const RuntimeAccessor::PV* pv = RuntimeAccessor::getPointerValue(val);
    result.data.pointer = const_cast<void*>(static_cast<const void*>(pv));
    // Null the Value's internal pointer so ~Value() won't invalidate the PV.
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

// ---------------------------------------------------------------------------
// Helper classes for HostObject, NativeState, ArrayBuffer, PreparedJS
// ---------------------------------------------------------------------------

class OwnedMutableBuffer : public jsi::MutableBuffer {
  std::vector<uint8_t> buf_;
 public:
  explicit OwnedMutableBuffer(size_t size) : buf_(size, 0) {}
  size_t size() const override { return buf_.size(); }
  uint8_t* data() override { return buf_.data(); }
};

class CNativeState : public jsi::NativeState {
  void* data_;
  HermesNativeStateFinalizer finalizer_;
 public:
  CNativeState(void* data, HermesNativeStateFinalizer fin)
      : data_(data), finalizer_(fin) {}
  ~CNativeState() override {
    if (finalizer_ && data_) finalizer_(data_);
  }
  void* data() const { return data_; }
};

class CHostObject : public jsi::HostObject {
  HermesRt* hrt_;
  HermesHostObjectGetCallback get_cb_;
  HermesHostObjectSetCallback set_cb_;
  HermesHostObjectGetPropertyNamesCallback get_names_cb_;
  void* user_data_;
  HermesHostObjectFinalizer finalizer_;

 public:
  CHostObject(HermesRt* hrt,
              HermesHostObjectGetCallback get_cb,
              HermesHostObjectSetCallback set_cb,
              HermesHostObjectGetPropertyNamesCallback get_names_cb,
              void* user_data,
              HermesHostObjectFinalizer finalizer)
      : hrt_(hrt), get_cb_(get_cb), set_cb_(set_cb),
        get_names_cb_(get_names_cb), user_data_(user_data),
        finalizer_(finalizer) {}

  ~CHostObject() override {
    if (finalizer_ && user_data_) {
      finalizer_(user_data_);
    }
  }

  void* userData() const { return user_data_; }

  jsi::Value get(jsi::Runtime& /*runtime*/,
                 const jsi::PropNameID& name) override {
    const void* name_pv = RuntimeAccessor::getPointerValue(name);
    HermesValue result = get_cb_(hrt_, name_pv, user_data_);
    switch (result.kind) {
      case HermesValueKind_Undefined: return jsi::Value::undefined();
      case HermesValueKind_Null: return jsi::Value(nullptr);
      case HermesValueKind_Boolean: return jsi::Value(result.data.boolean);
      case HermesValueKind_Number: return jsi::Value(result.data.number);
      case HermesValueKind_Symbol:
        return jsi::Value(RuntimeAccessor::make<jsi::Symbol>(
            static_cast<RuntimeAccessor::PV*>(result.data.pointer)));
      case HermesValueKind_BigInt:
        return jsi::Value(RuntimeAccessor::make<jsi::BigInt>(
            static_cast<RuntimeAccessor::PV*>(result.data.pointer)));
      case HermesValueKind_String:
        return jsi::Value(RuntimeAccessor::make<jsi::String>(
            static_cast<RuntimeAccessor::PV*>(result.data.pointer)));
      case HermesValueKind_Object:
        return jsi::Value(RuntimeAccessor::make<jsi::Object>(
            static_cast<RuntimeAccessor::PV*>(result.data.pointer)));
    }
    return jsi::Value::undefined();
  }

  void set(jsi::Runtime& /*runtime*/, const jsi::PropNameID& name,
           const jsi::Value& value) override {
    const void* name_pv = RuntimeAccessor::getPointerValue(name);
    HermesValue c_val;
    if (value.isUndefined()) {
      c_val.kind = HermesValueKind_Undefined;
      c_val.data.number = 0;
    } else if (value.isNull()) {
      c_val.kind = HermesValueKind_Null;
      c_val.data.number = 0;
    } else if (value.isBool()) {
      c_val.kind = HermesValueKind_Boolean;
      c_val.data.boolean = value.getBool();
    } else if (value.isNumber()) {
      c_val.kind = HermesValueKind_Number;
      c_val.data.number = value.getNumber();
    } else if (value.isString()) {
      c_val.kind = HermesValueKind_String;
      c_val.data.pointer = const_cast<void*>(
          static_cast<const void*>(RuntimeAccessor::getPointerValue(value)));
    } else if (value.isObject()) {
      c_val.kind = HermesValueKind_Object;
      c_val.data.pointer = const_cast<void*>(
          static_cast<const void*>(RuntimeAccessor::getPointerValue(value)));
    } else if (value.isSymbol()) {
      c_val.kind = HermesValueKind_Symbol;
      c_val.data.pointer = const_cast<void*>(
          static_cast<const void*>(RuntimeAccessor::getPointerValue(value)));
    } else if (value.isBigInt()) {
      c_val.kind = HermesValueKind_BigInt;
      c_val.data.pointer = const_cast<void*>(
          static_cast<const void*>(RuntimeAccessor::getPointerValue(value)));
    } else {
      c_val.kind = HermesValueKind_Undefined;
      c_val.data.number = 0;
    }
    set_cb_(hrt_, name_pv, &c_val, user_data_);
  }

  std::vector<jsi::PropNameID> getPropertyNames(
      jsi::Runtime& /*runtime*/) override {
    size_t count = 0;
    void** names = get_names_cb_(hrt_, &count, user_data_);
    std::vector<jsi::PropNameID> result;
    result.reserve(count);
    for (size_t i = 0; i < count; i++) {
      // Each entry is an OWNED PropNameID PV.
      result.push_back(RuntimeAccessor::make<jsi::PropNameID>(
          static_cast<RuntimeAccessor::PV*>(names[i])));
    }
    free(names);
    return result;
  }
};

struct HermesPreparedJs {
  std::shared_ptr<const jsi::PreparedJavaScript> prepared;
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

HermesRt* hermes__Runtime__NewWithConfig(const HermesRuntimeConfig* cfg) {
  auto builder = ::hermes::vm::RuntimeConfig::Builder()
      .withEnableEval(cfg->enable_eval)
      .withES6Proxy(cfg->es6_proxy)
      .withIntl(cfg->intl)
      .withMicrotaskQueue(cfg->microtask_queue)
      .withEnableGenerator(cfg->enable_generator)
      .withES6BlockScoping(cfg->enable_block_scoping)
      .withEnableHermesInternal(cfg->enable_hermes_internal)
      .withEnableHermesInternalTestMethods(cfg->enable_hermes_internal_test_methods)
      .withMaxNumRegisters(cfg->max_num_registers)
      .withEnableJIT(cfg->enable_jit)
      .withForceJIT(cfg->force_jit)
      .withJITThreshold(cfg->jit_threshold)
      .withJITMemoryLimit(cfg->jit_memory_limit)
      .withEnableAsyncGenerators(cfg->enable_async_generators)
      .withBytecodeWarmupPercent(cfg->bytecode_warmup_percent)
      .withRandomizeMemoryLayout(cfg->randomize_memory_layout);

  auto hrt = new HermesRt();
  hrt->runtime = facebook::hermes::makeHermesRuntime(builder.build());
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

void hermes__Runtime__SetPendingErrorMessage(HermesRt* hrt, const char* msg,
                                             size_t len) {
  free(hrt->pending_error_message);
  hrt->pending_error_message = static_cast<char*>(malloc(len + 1));
  memcpy(hrt->pending_error_message, msg, len);
  hrt->pending_error_message[len] = '\0';
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

bool hermes__Runtime__QueueMicrotask(HermesRt* hrt, const void* func) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Function> f(func);
  hrt->runtime->queueMicrotask(f.get());
  return true;
  HERMES_CATCH_BOOL(hrt)
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

// -- deleteProperty --

bool hermes__Object__DeleteProperty__String(
    HermesRt* hrt, const void* obj, const void* name) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  Borrowed<jsi::String> n(name);
  o.get().deleteProperty(rt(hrt), n.get());
  return true;
  HERMES_CATCH_BOOL(hrt)
}

bool hermes__Object__DeleteProperty__PropNameID(
    HermesRt* hrt, const void* obj, const void* name) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  Borrowed<jsi::PropNameID> n(name);
  o.get().deleteProperty(rt(hrt), n.get());
  return true;
  HERMES_CATCH_BOOL(hrt)
}

bool hermes__Object__DeleteProperty__Value(
    HermesRt* hrt, const void* obj, const struct HermesValue* name) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  jsi::Value key = c_to_jsi_value(rt(hrt), name);
  o.get().deleteProperty(rt(hrt), key);
  return true;
  HERMES_CATCH_BOOL(hrt)
}

// -- computed property access (Value key) --

struct HermesValue hermes__Object__GetProperty__Value(
    HermesRt* hrt, const void* obj, const struct HermesValue* name) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  jsi::Value key = c_to_jsi_value(rt(hrt), name);
  jsi::Value result = o.get().getProperty(rt(hrt), key);
  return jsi_value_to_c(std::move(result));
  HERMES_CATCH_VALUE(hrt)
}

bool hermes__Object__SetProperty__Value(
    HermesRt* hrt, const void* obj, const struct HermesValue* name,
    const struct HermesValue* val) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  jsi::Value key = c_to_jsi_value(rt(hrt), name);
  jsi::Value v = c_to_jsi_value(rt(hrt), val);
  o.get().setProperty(rt(hrt), key, std::move(v));
  return true;
  HERMES_CATCH_BOOL(hrt)
}

bool hermes__Object__HasProperty__Value(
    HermesRt* hrt, const void* obj, const struct HermesValue* name) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  jsi::Value key = c_to_jsi_value(rt(hrt), name);
  return o.get().hasProperty(rt(hrt), key);
  HERMES_CATCH_BOOL(hrt)
}

// -- prototype operations --

void* hermes__Object__CreateWithPrototype(
    HermesRt* hrt, const struct HermesValue* prototype) {
  HERMES_TRY(hrt)
  jsi::Value proto = c_to_jsi_value(rt(hrt), prototype);
  jsi::Object obj = jsi::Object::create(rt(hrt), proto);
  return steal_pointer(std::move(obj));
  HERMES_CATCH_PTR(hrt)
}

bool hermes__Object__SetPrototype(
    HermesRt* hrt, const void* obj, const struct HermesValue* prototype) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  jsi::Value proto = c_to_jsi_value(rt(hrt), prototype);
  o.get().setPrototype(rt(hrt), proto);
  return true;
  HERMES_CATCH_BOOL(hrt)
}

struct HermesValue hermes__Object__GetPrototype(
    HermesRt* hrt, const void* obj) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  jsi::Value proto = o.get().getPrototype(rt(hrt));
  return jsi_value_to_c(std::move(proto));
  HERMES_CATCH_VALUE(hrt)
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

int64_t hermes__BigInt__GetInt64(HermesRt* hrt, const void* bi) {
  Borrowed<jsi::BigInt> b(bi);
  return b.get().getInt64(rt(hrt));
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
// PropNameID (new)
// ---------------------------------------------------------------------------

void* hermes__PropNameID__ForSymbol(HermesRt* hrt, const void* sym) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Symbol> s(sym);
  jsi::PropNameID pni = jsi::PropNameID::forSymbol(rt(hrt), s.get());
  return steal_pointer(std::move(pni));
  HERMES_CATCH_PTR(hrt)
}

// ---------------------------------------------------------------------------
// Value (new)
// ---------------------------------------------------------------------------

void* hermes__Value__ToString(HermesRt* hrt, const struct HermesValue* val) {
  HERMES_TRY(hrt)
  jsi::Value v = c_to_jsi_value(rt(hrt), val);
  jsi::String str = v.toString(rt(hrt));
  return steal_pointer(std::move(str));
  HERMES_CATCH_PTR(hrt)
}

struct HermesValue hermes__Value__Clone(
    HermesRt* hrt, const struct HermesValue* val) {
  HERMES_TRY(hrt)
  jsi::Value v = c_to_jsi_value(rt(hrt), val);
  return jsi_value_to_c(std::move(v));
  HERMES_CATCH_VALUE(hrt)
}

// ---------------------------------------------------------------------------
// JSON
// ---------------------------------------------------------------------------

struct HermesValue hermes__Runtime__CreateValueFromJsonUtf8(
    HermesRt* hrt, const uint8_t* json, size_t len) {
  HERMES_TRY(hrt)
  jsi::Value v = jsi::Value::createFromJsonUtf8(rt(hrt), json, len);
  return jsi_value_to_c(std::move(v));
  HERMES_CATCH_VALUE(hrt)
}

// ---------------------------------------------------------------------------
// BigInt (new)
// ---------------------------------------------------------------------------

void* hermes__BigInt__ToString(HermesRt* hrt, const void* bi, int radix) {
  HERMES_TRY(hrt)
  Borrowed<jsi::BigInt> b(bi);
  jsi::String str = b.get().toString(rt(hrt), radix);
  return steal_pointer(std::move(str));
  HERMES_CATCH_PTR(hrt)
}

bool hermes__BigInt__StrictEquals(
    HermesRt* hrt, const void* a, const void* b) {
  Borrowed<jsi::BigInt> ba(a);
  Borrowed<jsi::BigInt> bb(b);
  return jsi::BigInt::strictEquals(rt(hrt), ba.get(), bb.get());
}

// ---------------------------------------------------------------------------
// ArrayBuffer
// ---------------------------------------------------------------------------

void* hermes__ArrayBuffer__New(HermesRt* hrt, size_t size) {
  HERMES_TRY(hrt)
  auto buffer = std::make_shared<OwnedMutableBuffer>(size);
  jsi::ArrayBuffer ab(*hrt->runtime, std::move(buffer));
  return steal_pointer(std::move(ab));
  HERMES_CATCH_PTR(hrt)
}

size_t hermes__ArrayBuffer__Size(HermesRt* hrt, const void* buf) {
  Borrowed<jsi::ArrayBuffer> ab(buf);
  return ab.get().size(rt(hrt));
}

uint8_t* hermes__ArrayBuffer__Data(HermesRt* hrt, const void* buf) {
  Borrowed<jsi::ArrayBuffer> ab(buf);
  return ab.get().data(rt(hrt));
}

// ---------------------------------------------------------------------------
// Object extensions (NativeState, HostObject, ExternalMemory)
// ---------------------------------------------------------------------------

void hermes__Object__SetExternalMemoryPressure(
    HermesRt* hrt, const void* obj, size_t amount) {
  Borrowed<jsi::Object> o(obj);
  o.get().setExternalMemoryPressure(rt(hrt), amount);
}

bool hermes__Object__HasNativeState(HermesRt* hrt, const void* obj) {
  Borrowed<jsi::Object> o(obj);
  return o.get().hasNativeState(rt(hrt));
}

void* hermes__Object__GetNativeState(HermesRt* hrt, const void* obj) {
  Borrowed<jsi::Object> o(obj);
  auto state = o.get().getNativeState<CNativeState>(rt(hrt));
  return state ? state->data() : nullptr;
}

void hermes__Object__SetNativeState(HermesRt* hrt, const void* obj,
    void* data, HermesNativeStateFinalizer finalizer) {
  HERMES_TRY(hrt)
  Borrowed<jsi::Object> o(obj);
  auto state = std::make_shared<CNativeState>(data, finalizer);
  o.get().setNativeState(rt(hrt), std::move(state));
  HERMES_CATCH_VOID(hrt)
}

void* hermes__Object__CreateFromHostObject(
    HermesRt* hrt,
    HermesHostObjectGetCallback get_cb,
    HermesHostObjectSetCallback set_cb,
    HermesHostObjectGetPropertyNamesCallback get_names_cb,
    void* user_data,
    HermesHostObjectFinalizer finalizer) {
  HERMES_TRY(hrt)
  auto ho = std::make_shared<CHostObject>(
      hrt, get_cb, set_cb, get_names_cb, user_data, finalizer);
  jsi::Object obj = jsi::Object::createFromHostObject(rt(hrt), std::move(ho));
  return steal_pointer(std::move(obj));
  HERMES_CATCH_PTR(hrt)
}

void* hermes__Object__GetHostObject(HermesRt* hrt, const void* obj) {
  Borrowed<jsi::Object> o(obj);
  auto ho = o.get().getHostObject<CHostObject>(rt(hrt));
  return ho ? ho->userData() : nullptr;
}

bool hermes__Object__IsHostObject(HermesRt* hrt, const void* obj) {
  Borrowed<jsi::Object> o(obj);
  return o.get().isHostObject(rt(hrt));
}

// ---------------------------------------------------------------------------
// PreparedJavaScript
// ---------------------------------------------------------------------------

HermesPreparedJs* hermes__Runtime__PrepareJavaScript(
    HermesRt* hrt, const uint8_t* data, size_t len,
    const char* url, size_t url_len) {
  HERMES_TRY(hrt)
  std::string source_url(url, url_len);
  auto buf = std::make_shared<jsi::StringBuffer>(
      std::string(reinterpret_cast<const char*>(data), len));
  auto prepared = hrt->runtime->prepareJavaScript(buf, std::move(source_url));
  auto result = new HermesPreparedJs();
  result->prepared = std::move(prepared);
  return result;
  HERMES_CATCH_PTR(hrt)
}

struct HermesValue hermes__Runtime__EvaluatePreparedJavaScript(
    HermesRt* hrt, const HermesPreparedJs* prepared) {
  HERMES_TRY(hrt)
  jsi::Value result =
      hrt->runtime->evaluatePreparedJavaScript(prepared->prepared);
  return jsi_value_to_c(std::move(result));
  HERMES_CATCH_VALUE(hrt)
}

void hermes__PreparedJavaScript__Delete(HermesPreparedJs* prepared) {
  delete prepared;
}

// ---------------------------------------------------------------------------
// Scope
// ---------------------------------------------------------------------------

void* hermes__Scope__New(HermesRt* hrt) {
  return new jsi::Scope(*hrt->runtime);
}

void hermes__Scope__Delete(void* scope) {
  delete static_cast<jsi::Scope*>(scope);
}

// ---------------------------------------------------------------------------
// Runtime info
// ---------------------------------------------------------------------------

size_t hermes__Runtime__Description(HermesRt* hrt, char* buf, size_t buf_len) {
  std::string desc = hrt->runtime->description();
  size_t needed = desc.size();
  if (buf && buf_len > 0) {
    size_t to_copy = needed < buf_len ? needed : buf_len;
    memcpy(buf, desc.data(), to_copy);
  }
  return needed;
}

bool hermes__Runtime__IsInspectable(HermesRt* hrt) {
  return hrt->runtime->isInspectable();
}

// ---------------------------------------------------------------------------
// Evaluate with source map
// ---------------------------------------------------------------------------

struct HermesValue hermes__Runtime__EvaluateJavaScriptWithSourceMap(
    HermesRt* hrt,
    const uint8_t* data, size_t len,
    const uint8_t* source_map, size_t source_map_len,
    const char* url, size_t url_len) {
  HERMES_TRY(hrt)
  std::string source_url(url, url_len);
  auto code_buf = std::make_shared<jsi::StringBuffer>(
      std::string(reinterpret_cast<const char*>(data), len));
  auto map_buf = std::make_shared<jsi::StringBuffer>(
      std::string(reinterpret_cast<const char*>(source_map), source_map_len));
  jsi::Value result = hrt->runtime->evaluateJavaScriptWithSourceMap(
      code_buf, map_buf, source_url);
  return jsi_value_to_c(std::move(result));
  HERMES_CATCH_VALUE(hrt)
}

// ---------------------------------------------------------------------------
// HermesRuntime-specific
// ---------------------------------------------------------------------------

static facebook::hermes::IHermesRootAPI* getRootAPI() {
  static auto* api = jsi::castInterface<facebook::hermes::IHermesRootAPI>(
      facebook::hermes::makeHermesRootAPI());
  return api;
}

static facebook::hermes::IHermes* getIHermes(HermesRt* hrt) {
  return jsi::castInterface<facebook::hermes::IHermes>(hrt->runtime.get());
}

bool hermes__IsHermesBytecode(const uint8_t* data, size_t len) {
  return getRootAPI()->isHermesBytecode(data, len);
}

uint32_t hermes__GetBytecodeVersion(void) {
  return getRootAPI()->getBytecodeVersion();
}

void hermes__PrefetchHermesBytecode(const uint8_t* data, size_t len) {
  getRootAPI()->prefetchHermesBytecode(data, len);
}

bool hermes__HermesBytecodeSanityCheck(const uint8_t* data, size_t len) {
  return getRootAPI()->hermesBytecodeSanityCheck(data, len);
}

void hermes__Runtime__WatchTimeLimit(HermesRt* hrt, uint32_t timeout_ms) {
  hrt->runtime->watchTimeLimit(timeout_ms);
}

void hermes__Runtime__UnwatchTimeLimit(HermesRt* hrt) {
  hrt->runtime->unwatchTimeLimit();
}

void hermes__Runtime__AsyncTriggerTimeout(HermesRt* hrt) {
  hrt->runtime->asyncTriggerTimeout();
}

void hermes__EnableSamplingProfiler(void) {
  getRootAPI()->enableSamplingProfiler();
}

void hermes__DisableSamplingProfiler(void) {
  getRootAPI()->disableSamplingProfiler();
}

void hermes__DumpSampledTraceToFile(const char* filename) {
  getRootAPI()->dumpSampledTraceToFile(std::string(filename));
}

// ---------------------------------------------------------------------------
// Fatal handler
// ---------------------------------------------------------------------------

static HermesFatalHandler g_fatal_handler = nullptr;

void hermes__SetFatalHandler(HermesFatalHandler handler) {
  g_fatal_handler = handler;
  if (handler) {
    getRootAPI()->setFatalHandler([](const std::string& msg) {
      if (g_fatal_handler) {
        g_fatal_handler(msg.data(), msg.size());
      }
    });
  } else {
    getRootAPI()->setFatalHandler(nullptr);
  }
}

// ---------------------------------------------------------------------------
// Bytecode epilogue
// ---------------------------------------------------------------------------

const uint8_t* hermes__GetBytecodeEpilogue(
    const uint8_t* data, size_t len, size_t* out_epilogue_len) {
  auto [ptr, size] = getRootAPI()->getBytecodeEpilogue(data, len);
  if (out_epilogue_len) *out_epilogue_len = size;
  return ptr;
}

// ---------------------------------------------------------------------------
// Code coverage profiler
// ---------------------------------------------------------------------------

bool hermes__IsCodeCoverageProfilerEnabled(void) {
  return getRootAPI()->isCodeCoverageProfilerEnabled();
}

void hermes__EnableCodeCoverageProfiler(void) {
  getRootAPI()->enableCodeCoverageProfiler();
}

void hermes__DisableCodeCoverageProfiler(void) {
  getRootAPI()->disableCodeCoverageProfiler();
}

// ---------------------------------------------------------------------------
// Per-runtime profiling
// ---------------------------------------------------------------------------

void hermes__Runtime__RegisterForProfiling(HermesRt* hrt) {
  getIHermes(hrt)->registerForProfiling();
}

void hermes__Runtime__UnregisterForProfiling(HermesRt* hrt) {
  getIHermes(hrt)->unregisterForProfiling();
}

// ---------------------------------------------------------------------------
// Load segment
// ---------------------------------------------------------------------------

bool hermes__Runtime__LoadSegment(
    HermesRt* hrt, const uint8_t* data, size_t len,
    const struct HermesValue* context) {
  HERMES_TRY(hrt)
  auto buf = std::make_unique<jsi::StringBuffer>(
      std::string(reinterpret_cast<const char*>(data), len));
  jsi::Value ctx = c_to_jsi_value(rt(hrt), context);
  getIHermes(hrt)->loadSegment(std::move(buf), ctx);
  return true;
  HERMES_CATCH_BOOL(hrt)
}

// ---------------------------------------------------------------------------
// Unique ID
// ---------------------------------------------------------------------------

uint64_t hermes__Object__GetUniqueID(HermesRt* hrt, const void* obj) {
  Borrowed<jsi::Object> o(obj);
  return getIHermes(hrt)->getUniqueID(o.get());
}

uint64_t hermes__String__GetUniqueID(HermesRt* hrt, const void* str) {
  Borrowed<jsi::String> s(str);
  return getIHermes(hrt)->getUniqueID(s.get());
}

uint64_t hermes__Symbol__GetUniqueID(HermesRt* hrt, const void* sym) {
  Borrowed<jsi::Symbol> s(sym);
  return getIHermes(hrt)->getUniqueID(s.get());
}

uint64_t hermes__BigInt__GetUniqueID(HermesRt* hrt, const void* bi) {
  Borrowed<jsi::BigInt> b(bi);
  return getIHermes(hrt)->getUniqueID(b.get());
}

uint64_t hermes__PropNameID__GetUniqueID(HermesRt* hrt, const void* pni) {
  Borrowed<jsi::PropNameID> p(pni);
  return getIHermes(hrt)->getUniqueID(p.get());
}

uint64_t hermes__Value__GetUniqueID(HermesRt* hrt,
                                     const struct HermesValue* val) {
  jsi::Value v = c_to_jsi_value(rt(hrt), val);
  return getIHermes(hrt)->getUniqueID(v);
}

// ---------------------------------------------------------------------------
// Reset timezone cache
// ---------------------------------------------------------------------------

void hermes__Runtime__ResetTimezoneCache(HermesRt* hrt) {
  getIHermes(hrt)->resetTimezoneCache();
}

} // extern "C"
