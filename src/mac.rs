extern crate libc;

use darwin_libproc;
use libc::{proc_listallpids, proc_pidpath};
use std::{ffi::CStr, ptr};

const PID_COUNT_MAX: usize = 100000;
const PATH_MAX: usize = 4096;

// global vector to store temp pids
static mut PID_BUFFER: [i32; 100000] = [0i32; PID_COUNT_MAX];
static mut PATH_BUFFER: [i8; PATH_MAX] = [0i8; PATH_MAX];

pub fn check_memory_usage(pid: i32) -> Result<u64, String> {
  let proc_info = darwin_libproc::task_info(pid);

  if proc_info.is_err() {
    return Err("Failed to get process info".to_string());
  } else {
    let memory_usage = proc_info.unwrap().pti_resident_size as u64;
    Ok(memory_usage)
  }
}

pub fn find_processes(starting_with: &str, pids: &mut [i32; 100000]) -> Result<(), String> {
  let mut index = 0;

  let buffer_size = unsafe { proc_listallpids(ptr::null_mut(), 0) };
  if buffer_size < 0 {
    return Err("Failed to get buffer size".to_string());
  }

  let buffer_size_used = unsafe {
    proc_listallpids(PID_BUFFER.as_mut_ptr() as *mut libc::c_void, buffer_size)
  };
  if buffer_size_used < 0 {
    return Err("Failed to get process list".to_string());
  }

  for &pid in unsafe { PID_BUFFER.iter() } {
    if pid <= 0 {
      continue;
    }

    let path_length = unsafe {
      proc_pidpath(pid, PATH_BUFFER.as_mut_ptr() as *mut libc::c_void, libc::PATH_MAX as u32)
    };

    if path_length > 0 {
      if let Ok(cstr) = unsafe { CStr::from_ptr(PATH_BUFFER.as_ptr()) }.to_str() {
        let procname = cstr.split('/').last().unwrap_or("");
        if procname.starts_with(starting_with) {
          if index >= PID_COUNT_MAX {
            return Err("Exceeded maximum number of processes".to_string());
          }
          pids[index] = pid;
          index += 1;
        }
      }
    }
  }

  Ok(())
}
