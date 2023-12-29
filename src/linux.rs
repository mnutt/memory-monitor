pub fn check_memory_usage(pid: i32) -> io::Result<u64, String> {
  // Path to the statm file
  let path = Path::new("/proc").join(pid.to_string()).join("statm");

  // Open the statm file
  let mut file = File::open(path)?;

  // Read the contents of the file
  let mut contents = [0; 24]; // 24 bytes is usually enough to read the first two numbers
  let _ = file.read(&mut contents)?;

  // Extract the resident set size (RSS) from the contents
  // RSS is typically the second number in the file
  let contents_str = std::str::from_utf8(&contents)
      .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
  let mut parts = contents_str.split_whitespace();
  let _ = parts.next(); // Skip the first number (total program size)
  if let Some(rss_str) = parts.next() {
      if let Ok(rss) = rss_str.parse::<u64>() {
          // Convert pages to bytes (assuming 4KB pages, which is typical)
          return Ok(rss * 4096);
      }
  }

  Err(io::Error::new(
      io::ErrorKind::Other,
      "Failed to parse memory usage",
  ))
}

const MAX_PROCESSES: usize = 1000; // Maximum number of processes to monitor

pub fn find_processes(process_name: &str) -> io::Result<[i32; MAX_PROCESSES]> {
    let mut pids = [0; MAX_PROCESSES];
    let mut count = 0;

    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        if let Ok(pid) = entry.file_name().to_string_lossy().parse::<i32>() {
            let cmdline_path = Path::new("/proc").join(pid.to_string()).join("cmdline");

            if let Ok(mut file) = fs::File::open(cmdline_path) {
                let mut cmdline = [0u8; 256]; // Buffer for command line
                let _ = file.read(&mut cmdline)?;
                if cmdline.starts_with(process_name.as_bytes()) {
                    if count < MAX_PROCESSES {
                        pids[count] = pid;
                        count += 1;
                    } else {
                        break;
                    }
                }
            }
        }
    }

    Ok(pids)
}
