//! Browser adapter for the kernel service (ARCHITECTURE.md, "Kernel service
//! and transport adapters"): the same openmat-kernel that backs the Tauri
//! command, compiled to wasm32 so the web build runs the real engine rather
//! than any JS reimplementation. Interface kept to plain C ABI + JSON so no
//! bindgen tooling is needed: strings cross as (pointer, length) pairs into
//! linear memory, and the result comes back as one length-prefixed JSON
//! buffer ([4-byte little-endian length][KernelResult JSON]).
//!
//! Memory contract with the JS side (app/src/engine/wasmEngine.ts):
//! - JS calls `om_alloc(len)` and writes each input string, then calls
//!   `om_evaluate`, which consumes and frees those input buffers.
//! - The returned result buffer is freed by JS via
//!   `om_free(ptr, 4 + json_len)` after reading.

use std::collections::HashMap;

/// Allocate `len` bytes inside wasm linear memory for the JS caller to
/// write into. Freed by `om_evaluate` (inputs) or `om_free` (results).
#[no_mangle]
pub extern "C" fn om_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Free a buffer previously handed out by `om_alloc`/`om_evaluate`.
///
/// # Safety
/// `ptr` must come from `om_alloc` or `om_evaluate` with exactly this `len`,
/// and must not be used again afterwards.
#[no_mangle]
pub unsafe extern "C" fn om_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        drop(Vec::from_raw_parts(ptr, 0, len));
    }
}

/// Evaluate one input through the shared kernel session. Takes the WL
/// source and a `{"name": number}` bindings object as UTF-8 buffers, and
/// returns a pointer to a length-prefixed KernelResult JSON buffer.
///
/// # Safety
/// Both (ptr, len) pairs must describe buffers obtained from `om_alloc` and
/// fully written by the caller. Ownership of both transfers here.
#[no_mangle]
pub unsafe extern "C" fn om_evaluate(
    input_ptr: *mut u8,
    input_len: usize,
    bindings_ptr: *mut u8,
    bindings_len: usize,
    request_id: u64,
) -> *mut u8 {
    let input_buf = Vec::from_raw_parts(input_ptr, input_len, input_len);
    let bindings_buf = Vec::from_raw_parts(bindings_ptr, bindings_len, bindings_len);

    let input = String::from_utf8_lossy(&input_buf);
    let bindings: HashMap<String, f64> =
        serde_json::from_slice(&bindings_buf).unwrap_or_default();

    let result = openmat_kernel::evaluate_with_bindings(&input, &bindings, request_id);
    let json = serde_json::to_string(&result).unwrap_or_else(|_| String::from("{}"));

    let bytes = json.as_bytes();
    let mut out = Vec::<u8>::with_capacity(4 + bytes.len());
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
    let ptr = out.as_mut_ptr();
    std::mem::forget(out);
    ptr
}
