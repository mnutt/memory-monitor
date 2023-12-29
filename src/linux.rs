use std::io;
use std::path::Path;
use std::fs::File;
use std::fs;
use std::io::Read;

static mut CONTENTS: [u8; 24] = [0; 24];

pub fn check_memory_usage(pid: i32) -> Result<u64, String> {
  // Path to the statm file
  let path = Path::new("/proc").join(pid.to_string()).join("statm");

  // Open the statm file
  let mut file = File::open(path).map_err(|e| e.to_string())?;

  // Read the contents of the file
   // 24 bytes is usually enough to read the first two numbers
  let _ = file.read(&mut CONTENTS).map_err(|e| e.to_string());

  // Extract the resident set size (RSS) from the contents
  // RSS is typically the second number in the file
  let contents_str = std::str::from_utf8(&CONTENTS)
      .map_err(|_| "Invalid data".to_string())?;
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

pub fn find_processes(starting_with: &str, pids: &mut [i32; 100000]) -> io::Result<()> {
    let mut count = 0;

    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        if let Ok(pid) = entry.file_name().to_string_lossy().parse::<i32>() {
            let cmdline_path = Path::new("/proc").join(pid.to_string()).join("cmdline");

            if let Ok(mut file) = fs::File::open(cmdline_path) {
                let path_buffer_slice: &mut [u8] = unsafe { &mut super::PATH_BUFFER };
                let _ = file.read(path_buffer_slice)?;
                let procpath = path_buffer_slice.split(|&c| c == b'\0').next().unwrap_or(&[b' ']);
                let procname = procpath.split(|&c| c == b'/').last().unwrap_or(&[b' ']);
                println!("procname: {:?}", procname);

                if procname.starts_with(starting_with.as_bytes()) {
                    if count < 100000 {
                        pids[count] = pid;
                        count += 1;
                    } else {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
