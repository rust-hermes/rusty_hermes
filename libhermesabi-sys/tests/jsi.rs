use libhermesabi_sys::*;
use std::ffi::CString;

unsafe extern "C" fn release_wrapper(_buf: *mut HermesABIBuffer) {}

unsafe extern "C" fn release_func_wrapper(_buf: *mut HermesABIHostFunction) {}

unsafe extern "C" fn add_func(
    _fn: *mut HermesABIHostFunction,
    _rt: *mut HermesABIRuntime,
    _this: *const HermesABIValue,
    args: *const HermesABIValue,
    _arg_count: usize,
) -> HermesABIValueOrError {
    let a = &*args;
    let b = &*(args.offset(1));

    HermesABIValueOrError {
        value: HermesABIValue {
            kind: HermesABIValueKind_HermesABIValueKindNumber,
            data: HermesABIValue__bindgen_ty_1 {
                number: a.data.number + b.data.number,
            },
        },
    }
}

#[test]
fn init_runtime() {
    unsafe {
        let vtable_ptr = get_hermes_abi_vtable();
        let vtable = &*vtable_ptr;

        let config = std::ptr::null();
        let runtime_ptr = (vtable.make_hermes_runtime.unwrap())(config);

        let runtime = &*runtime_ptr;
        let runtime_vt = &*runtime.vt;

        let script = String::from("add(1, 2)");
        let script_url = CString::new("./src/test.js").expect("CString::new failed");

        let vtable = HermesABIBufferVTable {
            release: Some(release_wrapper),
        };

        let mut x = HermesABIBuffer {
            vtable: &vtable,
            data: script.as_ptr(),
            size: script.len(),
        };

        let buffer_ptr = &mut x as *mut HermesABIBuffer;

        let global_obj = runtime_vt.get_global_object.unwrap()(runtime_ptr);
        let funcname_string =
            runtime_vt.create_string_from_utf8.unwrap()(runtime_ptr, "add".as_ptr(), "add".len());

        let funcname = runtime_vt.create_propnameid_from_string.unwrap()(
            runtime_ptr,
            HermesABIString {
                pointer: funcname_string.ptr_or_error as *mut HermesABIManagedPointer,
            },
        );

        let mut host_func = HermesABIHostFunction {
            vtable: &HermesABIHostFunctionVTable {
                call: Some(add_func),
                release: Some(release_func_wrapper),
            },
        };

        let add_func = runtime_vt.create_function_from_host_function.unwrap()(
            runtime_ptr,
            HermesABIPropNameID {
                pointer: funcname.ptr_or_error as *mut HermesABIManagedPointer,
            },
            2,
            &mut host_func,
        );

        runtime_vt.set_object_property_from_propnameid.unwrap()(
            runtime_ptr,
            global_obj,
            HermesABIPropNameID {
                pointer: funcname.ptr_or_error as *mut HermesABIManagedPointer,
            },
            &HermesABIValue {
                kind: HermesABIValueKind_HermesABIValueKindObject,
                data: HermesABIValue__bindgen_ty_1 {
                    pointer: add_func.ptr_or_error as *mut HermesABIManagedPointer,
                },
            },
        );

        let eval = runtime_vt.evaluate_javascript_source.unwrap();
        let v = eval(
            runtime_ptr,
            buffer_ptr,
            script_url.as_ptr(),
            script_url.as_bytes().len(),
        );

        assert_eq!(v.value.data.number, 3.0);
    }
}
