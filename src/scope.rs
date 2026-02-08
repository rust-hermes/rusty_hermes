use libhermes_sys::*;

use crate::Runtime;

/// RAII handle scope for managing GC pressure.
///
/// While a `Scope` is alive, all JS values created within it are kept alive.
/// When the scope drops, temporary values created within it become eligible
/// for garbage collection.
///
/// ```rust,no_run
/// use rusty_hermes::{Runtime, Scope};
///
/// let rt = Runtime::new().unwrap();
/// {
///     let _scope = Scope::new(&rt);
///     // values created here are scoped
/// }
/// // temporary values can now be GC'd
/// ```
pub struct Scope {
    raw: *mut std::ffi::c_void,
}

impl Scope {
    /// Push a new handle scope onto the runtime's scope stack.
    pub fn new(rt: &Runtime) -> Self {
        let raw = unsafe { hermes__Scope__New(rt.raw) };
        Scope { raw }
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        unsafe { hermes__Scope__Delete(self.raw) }
    }
}

impl std::fmt::Debug for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Scope({:?})", self.raw)
    }
}
