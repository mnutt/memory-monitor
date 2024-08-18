use clap::Parser;
use std::error::Error;
use std::thread;
use std::time::Duration;

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
    max_memory: u16,

    // Polling interval, in seconds
    #[arg(short, long, default_value = "2")]
    interval: u16,

    /// Signal to send to the process when memory threshold is exceeded
    #[arg(short, long, default_value = "SIGTERM")]
    signal: String,
}

#[cfg(target_os = "macos")]
use mac::{MemoryChecker, ProcDir};

#[cfg(target_os = "linux")]
use linux::{MemoryChecker, ProcDir};

// How many matched pids we can store
pub const PID_COUNT_MAX: usize = 5000;

fn signal_from_string(signal: &str) -> Option<i32> {
    match signal {
        "SIGUSR1" => Some(libc::SIGUSR1),
        "SIGUSR2" => Some(libc::SIGUSR2),
        "SIGTERM" => Some(libc::SIGTERM),
        "SIGKILL" => Some(libc::SIGKILL),
        _ => None,
    }
}

fn monitor_processes(starting_with: &str, memory_threshold: u64, interval: u16, signal: &str) -> Result<(), Box<dyn Error>> {
    let mut pids: Vec<i32> = Vec::with_capacity(PID_COUNT_MAX);

    let mut proc_dir = ProcDir::open()?;
    let mut checker = MemoryChecker::new();
    let sleep_duration = Duration::from_secs(interval as u64);
    let signal = signal_from_string(signal).unwrap_or(libc::SIGTERM);

    loop {
        println!("Checking for processes");

        if let Err(err) = proc_dir.find_processes(&mut pids, &starting_with) {
            eprintln!("Error finding processes: {}", err);
        }

        for &pid in pids.iter().filter(|&&pid| pid > 0) {
            println!("Checking memory usage for pid {}", pid);
            let usage = checker.get_memory(pid)?;
            if usage > memory_threshold {
                println!(
                    "  Memory usage for pid {} is {} MB, which is over the threshold of {} MB",
                    pid,
                    bytes_to_megabytes(usage),
                    bytes_to_megabytes(memory_threshold)
                );
                unsafe { libc::kill(pid as libc::pid_t, signal) };
            } else {
                println!(
                    "  Memory usage for pid {} is {} MB, which is under the threshold of {} MB",
                    pid,
                    bytes_to_megabytes(usage),
                    bytes_to_megabytes(memory_threshold)
                );
            }
        }
        thread::sleep(sleep_duration);
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
    let interval = cli.interval;
    let signal = cli.signal;

    println!(
        "Monitoring processes starting with {} that use more than {} MB of memory, polling every {} seconds, sending signal {}",
        process_name,
        bytes_to_megabytes(max_memory),
        interval,
        signal
    );

    monitor_processes(&process_name, max_memory, interval, &signal)
}
