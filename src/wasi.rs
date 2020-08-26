#![allow(clippy::box_vec, clippy::boxed_local)]

use crate::Result;
use std::{
    io::{self, Cursor},
    panic,
};
use tracing::{event, Level};

pub fn main() {
    panic::set_hook(Box::new(|info| {
        let s = format!("{}", info);
        unsafe {
            js_console_panic(s.as_ptr(), s.len());
        }
    }));
    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .with_writer(|| DebugWriter)
        .without_time()
        .json()
        .init();
}

extern "C" {
    fn js_console_panic(str_ptr: *const u8, str_len: usize);
    fn js_console_trace(str_ptr: *const u8, str_len: usize);
}

#[no_mangle]
pub extern "C" fn return_one() -> i32 {
    1
}

#[no_mangle]
pub extern "C" fn vec_u8_with_capacity(cap: usize) -> Box<Vec<u8>> {
    Box::new(Vec::with_capacity(cap))
}

#[no_mangle]
pub extern "C" fn vec_vec_with_capacity(cap: usize) -> Box<Vec<Box<Vec<u8>>>> {
    Box::new(Vec::with_capacity(cap))
}

#[no_mangle]
pub extern "C" fn vec_capacity(vec: &Vec<u8>) -> usize {
    vec.capacity()
}

#[no_mangle]
pub extern "C" fn vec_len(vec: &mut Vec<u8>) -> usize {
    vec.len()
}

#[no_mangle]
pub extern "C" fn vec_buffer(vec: &mut Vec<u8>) -> *mut u8 {
    vec.as_mut_ptr()
}

/// # Safety
/// Memory must be initialized before setting length and `len` <= `capacity`.
#[no_mangle]
pub unsafe extern "C" fn vec_set_len(vec: &mut Vec<u8>, index: usize) {
    vec.set_len(index);
}

#[no_mangle]
pub extern "C" fn vec_u8_drop(_vec: Box<Vec<u8>>) {}

/// # Safety
/// `date_now` must point to a UTF-8 encoded string.
#[no_mangle]
pub unsafe extern "C" fn jwl_merge(
    inputs: Box<Vec<Box<Vec<u8>>>>,
    date_now: Box<Vec<u8>>,
) -> Option<Box<MergeResult>> {
    let inputs = inputs.into_iter().map(|v| *v).collect();
    match merge(inputs, String::from_utf8_unchecked(*date_now)) {
        Ok(output) => Some(Box::new(output)),
        Err(e) => {
            event!(Level::ERROR, ?e, "Merge failed");
            None
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn merge_result_drop(_: Option<Box<MergeResult>>) {}

fn merge(inputs: Vec<Vec<u8>>, date_now: String) -> Result<MergeResult> {
    let max_size: usize = inputs.iter().map(|i| i.len()).sum();
    let inputs = inputs.into_iter().map(Cursor::new).collect();
    let (manifests, mem_file) = crate::run(inputs)?;
    let manifest = crate::update_manifest(&manifests, &mem_file, date_now);
    let mut output_file = Cursor::new(Vec::with_capacity(max_size / 3 * 2));
    crate::compress(&manifest, mem_file, &mut output_file)?;
    Ok(MergeResult {
        file: output_file.into_inner(),
        manifest: serde_json::to_vec(&manifest)?,
    })
}

#[repr(C)]
pub struct MergeResult {
    file: Vec<u8>,
    manifest: Vec<u8>,
}

struct DebugWriter;

impl io::Write for DebugWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe { js_console_trace(buf.as_ptr(), buf.len()) };
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::mem::size_of;

    #[test]
    fn test_layout_vec() {
        assert_eq!(size_of::<Vec<u8>>(), 3 * 32 / 8)
    }
}
