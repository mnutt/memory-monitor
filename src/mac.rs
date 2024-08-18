extern crate libc;

use darwin_libproc;
use libc::{proc_listallpids, proc_pidpath};
use std::{ffi::CStr, io};

pub const PID_COUNT_MAX: usize = super::PID_COUNT_MAX;

pub struct ProcDir {
    pid_buffer: [i32; PID_COUNT_MAX],
    path_buffer: [u8; libc::PATH_MAX as usize],
}

impl ProcDir {
    pub fn open() -> Result<Self, String> {
        Ok(Self {
            pid_buffer: [0i32; PID_COUNT_MAX],
            path_buffer: [0u8; libc::PATH_MAX as usize],
        })
    }

    pub fn find_processes(&mut self, pids: &mut Vec<i32>, starting_with: &str) -> io::Result<()> {
        pids.clear();

        let buffer_size_used = unsafe {
            proc_listallpids(
                self.pid_buffer.as_mut_ptr() as *mut libc::c_void,
                PID_COUNT_MAX as i32,
            )
        };
        if buffer_size_used < 0 {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Failed to get process list",
            ));
        }

        for &pid in self.pid_buffer.iter() {
            if pid <= 0 {
                continue;
            }

            let path_length = unsafe {
                proc_pidpath(
                    pid,
                    self.path_buffer.as_mut_ptr() as *mut libc::c_void,
                    libc::PATH_MAX as u32,
                )
            };

            if path_length > 0 {
                if let Ok(cstr) =
                    unsafe { CStr::from_ptr(self.path_buffer.as_ptr() as *const i8) }.to_str()
                {
                    let procname = cstr.split('/').last().unwrap_or("");
                    if procname.starts_with(starting_with) {
                        if pids.len() + 1 >= super::PID_COUNT_MAX {
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                "Exceeded maximum number of processes",
                            ));
                        }
                        pids.push(pid);
                    }
                }
            }
        }

        Ok(())
    }
}

pub struct MemoryChecker;

impl MemoryChecker {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_memory(&mut self, pid: i32) -> Result<u64, String> {
        let proc_info = darwin_libproc::task_info(pid);

        if proc_info.is_err() {
            return Err("Failed to get process info".to_string());
        } else {
            let memory_usage = proc_info.unwrap().pti_resident_size as u64;
            Ok(memory_usage)
        }
    }
}
