// Stable C bindings for Hermes JavaScript engine (rusty_v8 style).
// This header declares a flat extern "C" API wrapping the JSI C++ interface.

#ifndef HERMES_BINDING_HPP
#define HERMES_BINDING_HPP

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque runtime handle. Wraps HermesRuntime + exception state.
typedef struct HermesRt HermesRt;

// Value kind enum, mirrors jsi::Value::ValueKind.
enum HermesValueKind {
  HermesValueKind_Undefined = 0,
  HermesValueKind_Null = 1,
  HermesValueKind_Boolean = 2,
  HermesValueKind_Number = 3,
  HermesValueKind_Symbol = 4,
  HermesValueKind_BigInt = 5,
  HermesValueKind_String = 6,
  HermesValueKind_Object = 7,
};

// C-compatible tagged union mirroring jsi::Value.
// Pointer kinds hold a PointerValue* that must be released.
struct HermesValue {
  enum HermesValueKind kind;
  union {
    bool boolean;
    double number;
    void* pointer; // jsi::Runtime::PointerValue*, needs explicit release
  } data;
};

// HostFunction callback signature.
// Returns a HermesValue. On error, the callback should set the runtime's
// pending error state via hermes__Runtime__SetError and return undefined.
typedef struct HermesValue (*HermesHostFunctionCallback)(
    HermesRt* rt,
    const struct HermesValue* this_val,
    const struct HermesValue* args,
    size_t arg_count,
    void* user_data);

// Called when the host function is garbage collected.
typedef void (*HermesHostFunctionFinalizer)(void* user_data);

// ---------------------------------------------------------------------------
// Runtime lifecycle
// ---------------------------------------------------------------------------

HermesRt* hermes__Runtime__New(void);
void hermes__Runtime__Delete(HermesRt* rt);

// Exception state
bool hermes__Runtime__HasPendingError(const HermesRt* rt);
struct HermesValue hermes__Runtime__GetAndClearError(HermesRt* rt);
const char* hermes__Runtime__GetAndClearErrorMessage(HermesRt* rt);

// Global object (returns Object PointerValue*)
void* hermes__Runtime__Global(HermesRt* rt);

// ---------------------------------------------------------------------------
// Evaluate
// ---------------------------------------------------------------------------

struct HermesValue hermes__Runtime__EvaluateJavaScript(
    HermesRt* rt,
    const uint8_t* data,
    size_t len,
    const char* source_url,
    size_t source_url_len);

// Returns: 1 = drained, 0 = more work, -1 = error (check pending error)
int hermes__Runtime__DrainMicrotasks(HermesRt* rt, int max_hint);

// ---------------------------------------------------------------------------
// String
// ---------------------------------------------------------------------------

void* hermes__String__CreateFromUtf8(
    HermesRt* rt,
    const uint8_t* utf8,
    size_t len);

void* hermes__String__CreateFromAscii(
    HermesRt* rt,
    const char* ascii,
    size_t len);

// Writes UTF-8 into buf. Returns the number of bytes needed (excluding NUL).
// If buf is NULL or buf_len is 0, just returns the required size.
size_t hermes__String__ToUtf8(
    HermesRt* rt,
    const void* str,
    char* buf,
    size_t buf_len);

bool hermes__String__StrictEquals(
    HermesRt* rt,
    const void* a,
    const void* b);

void hermes__String__Release(void* pv);

// ---------------------------------------------------------------------------
// PropNameID
// ---------------------------------------------------------------------------

void* hermes__PropNameID__ForAscii(HermesRt* rt, const char* str, size_t len);
void* hermes__PropNameID__ForUtf8(
    HermesRt* rt,
    const uint8_t* utf8,
    size_t len);
void* hermes__PropNameID__ForString(HermesRt* rt, const void* str);

size_t hermes__PropNameID__ToUtf8(
    HermesRt* rt,
    const void* pni,
    char* buf,
    size_t buf_len);

bool hermes__PropNameID__Equals(
    HermesRt* rt,
    const void* a,
    const void* b);

void hermes__PropNameID__Release(void* pv);

// ---------------------------------------------------------------------------
// Object
// ---------------------------------------------------------------------------

void* hermes__Object__New(HermesRt* rt);

struct HermesValue hermes__Object__GetProperty__String(
    HermesRt* rt,
    const void* obj,
    const void* name);

struct HermesValue hermes__Object__GetProperty__PropNameID(
    HermesRt* rt,
    const void* obj,
    const void* name);

bool hermes__Object__SetProperty__String(
    HermesRt* rt,
    const void* obj,
    const void* name,
    const struct HermesValue* val);

bool hermes__Object__SetProperty__PropNameID(
    HermesRt* rt,
    const void* obj,
    const void* name,
    const struct HermesValue* val);

bool hermes__Object__HasProperty__String(
    HermesRt* rt,
    const void* obj,
    const void* name);

bool hermes__Object__HasProperty__PropNameID(
    HermesRt* rt,
    const void* obj,
    const void* name);

void* hermes__Object__GetPropertyNames(HermesRt* rt, const void* obj);

bool hermes__Object__IsArray(HermesRt* rt, const void* obj);
bool hermes__Object__IsFunction(HermesRt* rt, const void* obj);
bool hermes__Object__IsArrayBuffer(HermesRt* rt, const void* obj);

bool hermes__Object__StrictEquals(
    HermesRt* rt,
    const void* a,
    const void* b);

bool hermes__Object__InstanceOf(
    HermesRt* rt,
    const void* obj,
    const void* func);

void hermes__Object__Release(void* pv);

// ---------------------------------------------------------------------------
// Array
// ---------------------------------------------------------------------------

void* hermes__Array__New(HermesRt* rt, size_t length);
size_t hermes__Array__Size(HermesRt* rt, const void* arr);

struct HermesValue hermes__Array__GetValueAtIndex(
    HermesRt* rt,
    const void* arr,
    size_t index);

bool hermes__Array__SetValueAtIndex(
    HermesRt* rt,
    const void* arr,
    size_t index,
    const struct HermesValue* val);

void hermes__Array__Release(void* pv);

// ---------------------------------------------------------------------------
// Function
// ---------------------------------------------------------------------------

struct HermesValue hermes__Function__Call(
    HermesRt* rt,
    const void* func,
    const struct HermesValue* this_val,
    const struct HermesValue* args,
    size_t argc);

struct HermesValue hermes__Function__CallAsConstructor(
    HermesRt* rt,
    const void* func,
    const struct HermesValue* args,
    size_t argc);

void* hermes__Function__CreateFromHostFunction(
    HermesRt* rt,
    const void* name,
    unsigned int param_count,
    HermesHostFunctionCallback callback,
    void* user_data,
    HermesHostFunctionFinalizer finalizer);

bool hermes__Function__IsHostFunction(HermesRt* rt, const void* func);

void hermes__Function__Release(void* pv);

// ---------------------------------------------------------------------------
// Value
// ---------------------------------------------------------------------------

void hermes__Value__Release(struct HermesValue* val);
bool hermes__Value__StrictEquals(
    HermesRt* rt,
    const struct HermesValue* a,
    const struct HermesValue* b);

// ---------------------------------------------------------------------------
// Symbol
// ---------------------------------------------------------------------------

void* hermes__Symbol__ToString(HermesRt* rt, const void* sym);
bool hermes__Symbol__StrictEquals(
    HermesRt* rt,
    const void* a,
    const void* b);
void hermes__Symbol__Release(void* pv);

// ---------------------------------------------------------------------------
// BigInt
// ---------------------------------------------------------------------------

void* hermes__BigInt__FromInt64(HermesRt* rt, int64_t val);
void* hermes__BigInt__FromUint64(HermesRt* rt, uint64_t val);
bool hermes__BigInt__IsInt64(HermesRt* rt, const void* bi);
bool hermes__BigInt__IsUint64(HermesRt* rt, const void* bi);
uint64_t hermes__BigInt__Truncate(HermesRt* rt, const void* bi);
void hermes__BigInt__Release(void* pv);

// ---------------------------------------------------------------------------
// WeakObject
// ---------------------------------------------------------------------------

void* hermes__WeakObject__Create(HermesRt* rt, const void* obj);
struct HermesValue hermes__WeakObject__Lock(HermesRt* rt, const void* wo);
void hermes__WeakObject__Release(void* pv);

// ---------------------------------------------------------------------------
// HermesRuntime-specific (static)
// ---------------------------------------------------------------------------

bool hermes__IsHermesBytecode(const uint8_t* data, size_t len);
uint32_t hermes__GetBytecodeVersion(void);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // HERMES_BINDING_HPP
