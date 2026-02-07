use libhermesabi_sys::*;

/// A pre-compiled JavaScript script.
///
/// Parse/compile once, evaluate many times for better performance.
///
/// Created via [`Runtime::prepare_javascript`](crate::Runtime::prepare_javascript).
pub struct PreparedJavaScript {
    pub(crate) raw: *mut HermesPreparedJs,
}

impl Drop for PreparedJavaScript {
    fn drop(&mut self) {
        unsafe { hermes__PreparedJavaScript__Delete(self.raw) }
    }
}

impl std::fmt::Debug for PreparedJavaScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PreparedJavaScript({:?})", self.raw)
    }
}
