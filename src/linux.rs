use super::{MemoryChecker, ProcDir};
use libc;
use std::ffi::CStr;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;

pub const PATH_MAX: usize = 4096;

pub struct LinuxProcDir {
    dir: *mut libc::DIR,
    pidfile_buffer: Vec<u8>,
    procname_buffer: Vec<u8>,
}

impl ProcDir for LinuxProcDir {
    // Open /proc fd and initialize preallocated buffer for reads
    fn open() -> Result<Self, io::Error> {
        let dir = unsafe { libc::opendir(b"/proc\0".as_ptr() as *const i8) };
        if dir.is_null() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Failed to open /proc",
            ));
        }

        let pidfile_buffer = Vec::with_capacity(PATH_MAX);
        let procname_buffer = Vec::with_capacity(PATH_MAX);
        Ok(Self {
            dir,
            pidfile_buffer,
            procname_buffer,
        })
    }

    fn find_processes(&mut self, pids: &mut Vec<i32>, starting_with: &str) -> io::Result<()> {
        pids.clear();

        self.rewind();

        while let Some(pid) = self.next() {
            if let Ok(mut file) = open_proc_file(pid, "comm", &mut self.procname_buffer) {
                self.procname_buffer.clear();
                self.procname_buffer.resize(PATH_MAX, 0);

                if let Err(e) = file.read(&mut self.procname_buffer) {
                    return Err(e);
                }

                let contents = self
                    .procname_buffer
                    .split(|&c| c == b'\0')
                    .next()
                    .unwrap_or(&[b' ']);
                let procname = contents.split(|&c| c == b'\n').next().unwrap_or(&[b' ']);

                if procname.starts_with(starting_with.as_bytes()) {
                    pids.push(pid);
                }
            }
        }

        Ok(())
    }
}

impl LinuxProcDir {
    fn rewind(&mut self) {
        unsafe {
            libc::rewinddir(self.dir);
        }
    }
}

impl Iterator for LinuxProcDir {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        // Multiple times, to skip over entries that are not pids
        loop {
            unsafe {
                let entry = libc::readdir(self.dir);
                if entry.is_null() {
                    return None;
                }

                let d_name = (*entry).d_name.as_ptr();
                let name = CStr::from_ptr(d_name).to_bytes();

                self.pidfile_buffer.clear();
                self.pidfile_buffer.extend_from_slice(name);
            }

            if let Ok(name_str) = std::str::from_utf8(&self.pidfile_buffer) {
                if let Ok(pid) = name_str.parse::<i32>() {
                    return Some(pid);
                }
            }
        }
    }
}

impl Drop for LinuxProcDir {
    fn drop(&mut self) {
        if !self.dir.is_null() {
            unsafe {
                libc::closedir(self.dir);
            }
        }
    }
}

pub fn open_proc_file(pid: i32, filename: &'static str, buffer: &mut Vec<u8>) -> io::Result<File> {
    buffer.clear();
    write!(buffer, "/proc/{}/{}", pid, filename).unwrap();

    let path = Path::new(std::str::from_utf8(&buffer).unwrap());
    File::open(path)
}

pub struct LinuxMemoryChecker {
    buffer: Vec<u8>,
}

impl MemoryChecker for LinuxMemoryChecker {
    fn new() -> Self {
        let buffer = Vec::with_capacity(PATH_MAX);
        Self { buffer }
    }

    fn get_memory(&mut self, pid: i32) -> Result<u64, String> {
        // Open the statm file
        let mut file = open_proc_file(pid, "statm", &mut self.buffer).map_err(|e| e.to_string())?;

        // Read the contents of the file
        // 24 bytes is usually enough to read the first two numbers
        self.buffer.clear();
        self.buffer.resize(PATH_MAX, 0);
        let _ = file.read(&mut self.buffer).map_err(|e| e.to_string());

        // Extract the resident set size (RSS) from the contents
        // RSS is typically the second number in the file
        let contents_str =
            std::str::from_utf8(&self.buffer).map_err(|_| "Invalid data".to_string())?;

        let mut parts = contents_str.split_whitespace();
        let _ = parts.next(); // Skip the first number (total program size)
        if let Some(rss_str) = parts.next() {
            if let Ok(rss) = rss_str.parse::<u64>() {
                // Convert pages to bytes (assuming 4KB pages, which is typical)
                return Ok(rss * 4096);
            }
        }

        Err("Failed to get process info".to_string())
    }

    fn kill(&self, pid: libc::pid_t, signal: i32) {
        unsafe {
            libc::kill(pid, signal);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_memory_checker() {
        let mut checker = LinuxMemoryChecker::new();
        let pid = unsafe { libc::getpid() };
        let usage = checker.get_memory(pid).unwrap();
        assert!(usage > 0);
    }

    #[test]
    fn test_linux_proc_dir_no_matches() {
        let mut proc_dir = LinuxProcDir::open().unwrap();
        let mut pids = Vec::new();
        proc_dir
            .find_processes(&mut pids, "shouldneverfindthisprocess")
            .unwrap();
        assert!(pids.is_empty());
    }

    #[test]
    fn test_linux_proc_dir_with_matches() {
        let mut proc_dir = LinuxProcDir::open().unwrap();
        let mut pids = Vec::new();
        proc_dir.find_processes(&mut pids, "cargo").unwrap();
        assert!(!pids.is_empty());
    }
}
