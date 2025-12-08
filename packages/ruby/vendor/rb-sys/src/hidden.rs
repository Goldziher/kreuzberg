//! Hidden symbols from Ruby that we need to link to, not exposed to users.
//!
//! Note: Using these symbols is an absolute last resort. Try to use the
//! official Ruby C API if at all possible.

extern "C" {
    /// A pointer to the current Ruby VM.
    #[cfg(all(ruby_gt_2_4, ruby_lte_3_2))]
    #[cfg(ruby_engine = "mri")]
    pub(crate) static ruby_current_vm_ptr: *mut crate::ruby_vm_t;
}
