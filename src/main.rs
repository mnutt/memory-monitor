use std::thread;
use std::time::Duration;
use std::error::Error;
use clap::Parser;

#[cfg(target_os = "macos")]
mod mac;

#[cfg(target_os = "linux")]
mod linux;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Name of the process(es) to monitor
    name: String,

    /// Max memory, in MB
    #[arg(short, long)]
    max_memory: u8,
}

#[cfg(target_os = "macos")]
use mac::{check_memory_usage as check_memory_usage, find_processes as find_processes};

#[cfg(target_os = "linux")]
use linux::{check_memory_usage as check_memory_usage, find_processes as find_processes};

fn monitor_processes(starting_with: &str, memory_threshold: u64) -> Result<(), Box<dyn Error>> {
    let mut pids: [i32; 100000] = [0; 100000];
    loop {
        println!("Checking for processes");
        // Reset the pids array
        for pid in pids.iter_mut() {
            *pid = 0;
        }
        if let Err(err) = find_processes(&starting_with, &mut pids) {
            eprintln!("Error finding processes: {}", err);
        }

        for &pid in pids.iter().filter(|&&pid| pid > 0) {
            println!("Checking memory usage for pid {}", pid);
            let usage = check_memory_usage(pid)?;
            if usage > memory_threshold {
                println!("  Memory usage for pid {} is {} MB, which is over the threshold of {} MB", pid, bytes_to_megabytes(usage), bytes_to_megabytes(memory_threshold));
                unsafe { libc::kill(pid as libc::pid_t, libc::SIGUSR1) };
            } else {
                println!("  Memory usage for pid {} is {} MB, which is under the threshold of {} MB", pid, bytes_to_megabytes(usage), bytes_to_megabytes(memory_threshold));
            }
            println!("  Got past there");
        }
        thread::sleep(Duration::from_secs(2));
    }
}

fn bytes_to_megabytes(bytes: u64) -> u64 {
    bytes / 1024 / 1024
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting memory monitor");
    let cli = Cli::parse();

    let process_name = cli.name;
    let max_memory: u64 = cli.max_memory as u64 * 1024 * 1024;

    println!("Monitoring processes starting with {} that use more than {} MB of memory", process_name, max_memory / 1024 / 1024);

    let _ = monitor_processes(&process_name, max_memory);

    Ok(())
}
