#![allow(clippy::box_vec, clippy::boxed_local)]

use crate::{Manifest, Message, Progress, Result};
use serde::Serialize;
use std::{
    io::{self, Cursor},
    panic,
};
pub fn main() {
    panic::set_hook(Box::new(|info| {
        let s = format!("{}", info);
        unsafe {
            js_console_panic(s.as_ptr(), s.len());
        }
    }));
    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::WARN)
        .with_writer(|| DebugWriter)
        .without_time()
        .json()
        .init();
}

extern "C" {
    fn js_console_panic(str_ptr: *const u8, str_len: usize);
    fn js_console_trace(str_ptr: *const u8, str_len: usize);
    fn js_merge_progress(progress: Progress);
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
) -> Box<JsMerge> {
    let inputs = inputs.into_iter().map(|v| *v).collect();
    Box::new(
        match merge(inputs, String::from_utf8_unchecked(*date_now)) {
            Ok(output) => JsMerge {
                file: Some(output.0),
                result: serde_json::to_vec(&output.1).unwrap(),
            },
            Err(e) => JsMerge {
                file: None,
                result: serde_json::to_vec(&[Message::Error(format!("{:?}", e))]).unwrap(),
            },
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn merge_result_drop(_: Option<Box<JsMerge>>) {}

fn merge(inputs: Vec<Vec<u8>>, date_now: String) -> Result<(Vec<u8>, Json)> {
    let max_size: usize = inputs.iter().map(|i| i.len()).sum();
    let inputs = inputs.into_iter().map(Cursor::new).collect();
    let merge = crate::run(inputs, |p| unsafe { js_merge_progress(p) })?;
    let manifest = crate::update_manifest(&merge, date_now);
    let mut output_file = Cursor::new(Vec::with_capacity(max_size / 3 * 2));
    crate::compress(&manifest, merge.mem_file, &mut output_file)?;
    Ok((
        output_file.into_inner(),
        Json {
            manifest,
            messages: merge.messages,
        },
    ))
}

#[repr(C)]
pub struct JsMerge {
    file: Option<Vec<u8>>,
    result: Vec<u8>,
}

#[derive(Serialize)]
struct Json {
    manifest: Manifest,
    messages: Vec<Message>,
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
    use super::*;
    use std::mem::size_of;

    #[no_mangle]
    pub unsafe extern "C" fn js_merge_progress(_: super::Progress) {}

    #[test]
    fn test_layout_vec() {
        assert_eq!(size_of::<Vec<u8>>(), 3 * 32 / 8);
        assert_eq!(size_of::<Option<Vec<u8>>>(), 12);
        let none: Option<Vec<u8>> = None;
        unsafe {
            assert_eq!(*(&none as *const _ as *const i32), 0);
        }
    }

    #[test]
    fn test_json_message() {
        let json = serde_json::to_string(&Message::BookmarkOverflow {
            key_symbol: None,
            issue_tag_number: 123,
            title: "title".to_string().into(),
            snippet: Some("snip".to_string().into()),
        })
        .unwrap();
        dbg!(json);
    }
}
