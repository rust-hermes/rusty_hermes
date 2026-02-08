use std::marker::PhantomData;

use libhermesabi_sys::*;

use crate::error::{check_error, Error, Result};
use crate::propnameid::PropNameId;
use crate::value::Value;
use crate::{Array, Runtime};

/// A JavaScript object handle.
pub struct Object<'rt> {
    pub(crate) pv: *mut std::ffi::c_void,
    pub(crate) rt: *mut HermesRt,
    pub(crate) _marker: PhantomData<&'rt ()>,
}

impl<'rt> Object<'rt> {
    /// Create a new empty JS object.
    pub fn new(rt: &'rt Runtime) -> Self {
        let pv = unsafe { hermes__Object__New(rt.raw) };
        Object {
            pv,
            rt: rt.raw,
            _marker: PhantomData,
        }
    }

    // -- property access (string keys) -----------------------------------------

    /// Get a property by name.
    pub fn get(&self, key: &str) -> Result<Value<'rt>> {
        let key_pv = unsafe {
            hermes__String__CreateFromUtf8(self.rt, key.as_ptr(), key.len())
        };
        let raw = unsafe {
            hermes__Object__GetProperty__String(self.rt, self.pv, key_pv)
        };
        unsafe { hermes__String__Release(key_pv) };
        check_error(self.rt)?;
        Ok(unsafe { Value::from_raw(self.rt, raw) })
    }

    /// Set a property by name.
    pub fn set(&self, key: &str, val: Value<'rt>) -> Result<()> {
        let key_pv = unsafe {
            hermes__String__CreateFromUtf8(self.rt, key.as_ptr(), key.len())
        };
        let ok = unsafe {
            hermes__Object__SetProperty__String(self.rt, self.pv, key_pv, &val.raw)
        };
        unsafe { hermes__String__Release(key_pv) };
        if !ok {
            return check_error(self.rt).map(|_| ());
        }
        Ok(())
    }

    /// Check whether a property exists.
    pub fn has(&self, key: &str) -> bool {
        let key_pv = unsafe {
            hermes__String__CreateFromUtf8(self.rt, key.as_ptr(), key.len())
        };
        let result = unsafe {
            hermes__Object__HasProperty__String(self.rt, self.pv, key_pv)
        };
        unsafe { hermes__String__Release(key_pv) };
        result
    }

    // -- property access (PropNameID keys) ------------------------------------

    /// Get a property using a [`PropNameId`] key.
    pub fn get_with_propname(&self, key: &PropNameId<'rt>) -> Result<Value<'rt>> {
        let raw = unsafe {
            hermes__Object__GetProperty__PropNameID(self.rt, self.pv, key.pv)
        };
        check_error(self.rt)?;
        Ok(unsafe { Value::from_raw(self.rt, raw) })
    }

    /// Set a property using a [`PropNameId`] key.
    pub fn set_with_propname(
        &self,
        key: &PropNameId<'rt>,
        val: Value<'rt>,
    ) -> Result<()> {
        let ok = unsafe {
            hermes__Object__SetProperty__PropNameID(self.rt, self.pv, key.pv, &val.raw)
        };
        if !ok {
            return check_error(self.rt).map(|_| ());
        }
        Ok(())
    }

    /// Check whether a property exists using a [`PropNameId`] key.
    pub fn has_with_propname(&self, key: &PropNameId<'rt>) -> bool {
        unsafe {
            hermes__Object__HasProperty__PropNameID(self.rt, self.pv, key.pv)
        }
    }

    // -- host object support ---------------------------------------------------

    /// Create a JS object backed by a HostObject with custom get/set/getPropertyNames callbacks.
    ///
    /// # Safety
    /// The caller must ensure callback function pointers and `user_data` remain valid
    /// until the `finalizer` is called.
    pub unsafe fn create_host_object(
        rt: &'rt Runtime,
        get_cb: HermesHostObjectGetCallback,
        set_cb: HermesHostObjectSetCallback,
        get_names_cb: HermesHostObjectGetPropertyNamesCallback,
        user_data: *mut std::ffi::c_void,
        finalizer: HermesHostObjectFinalizer,
    ) -> Self {
        let pv = hermes__Object__CreateFromHostObject(
            rt.raw,
            get_cb,
            set_cb,
            get_names_cb,
            user_data,
            finalizer,
        );
        Object {
            pv,
            rt: rt.raw,
            _marker: PhantomData,
        }
    }

    /// Get the opaque user_data pointer from a HostObject, or null if not a HostObject.
    pub fn get_host_object_data(&self) -> *mut std::ffi::c_void {
        unsafe { hermes__Object__GetHostObject(self.rt, self.pv) }
    }

    /// Get all own property names as an [`Array`].
    pub fn property_names(&self) -> Result<Array<'rt>> {
        let arr_pv = unsafe { hermes__Object__GetPropertyNames(self.rt, self.pv) };
        check_error(self.rt)?;
        Ok(Array {
            pv: arr_pv,
            rt: self.rt,
            _marker: PhantomData,
        })
    }

    // -- type checks -----------------------------------------------------------

    pub fn is_array(&self) -> bool {
        unsafe { hermes__Object__IsArray(self.rt, self.pv) }
    }

    pub fn is_function(&self) -> bool {
        unsafe { hermes__Object__IsFunction(self.rt, self.pv) }
    }

    pub fn is_array_buffer(&self) -> bool {
        unsafe { hermes__Object__IsArrayBuffer(self.rt, self.pv) }
    }

    pub fn strict_equals(&self, other: &Object<'rt>) -> bool {
        unsafe { hermes__Object__StrictEquals(self.rt, self.pv, other.pv) }
    }

    pub fn instance_of(&self, func: &Object<'rt>) -> bool {
        unsafe { hermes__Object__InstanceOf(self.rt, self.pv, func.pv) }
    }

    /// Hint to the GC about external memory associated with this object.
    pub fn set_external_memory_pressure(&self, amount: usize) {
        unsafe {
            hermes__Object__SetExternalMemoryPressure(self.rt, self.pv, amount)
        }
    }

    /// Check if this object has attached native state.
    pub fn has_native_state(&self) -> bool {
        unsafe { hermes__Object__HasNativeState(self.rt, self.pv) }
    }

    /// Get the native state pointer, or null if none.
    pub fn get_native_state(&self) -> *mut std::ffi::c_void {
        unsafe { hermes__Object__GetNativeState(self.rt, self.pv) }
    }

    /// Attach native state to this object. The `finalizer` is called when the
    /// object is garbage collected.
    ///
    /// # Safety
    /// The caller must ensure `data` remains valid until the finalizer is called.
    pub unsafe fn set_native_state(
        &self,
        data: *mut std::ffi::c_void,
        finalizer: HermesNativeStateFinalizer,
    ) {
        hermes__Object__SetNativeState(self.rt, self.pv, data, finalizer);
    }

    /// Check if this object is a HostObject.
    pub fn is_host_object(&self) -> bool {
        unsafe { hermes__Object__IsHostObject(self.rt, self.pv) }
    }
}

impl Drop for Object<'_> {
    fn drop(&mut self) {
        unsafe { hermes__Object__Release(self.pv) }
    }
}

// -- Into<Value> / TryFrom<Value> ---------------------------------------------

impl<'rt> From<Object<'rt>> for Value<'rt> {
    fn from(obj: Object<'rt>) -> Value<'rt> {
        let val = Value {
            raw: HermesValue {
                kind: HermesValueKind_Object,
                data: HermesValueData { pointer: obj.pv },
            },
            rt: obj.rt,
            _marker: PhantomData,
        };
        std::mem::forget(obj);
        val
    }
}

impl<'rt> TryFrom<Value<'rt>> for Object<'rt> {
    type Error = Error;
    fn try_from(val: Value<'rt>) -> Result<Self> {
        val.into_object()
    }
}

impl std::fmt::Debug for Object<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Object({:?})", self.pv)
    }
}
