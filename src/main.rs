use clap::Parser;
use std::error::Error;
use std::thread;
use std::time::Duration;
use logging::Logger;

mod logging;

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

pub const PID_COUNT_MAX: usize = 100000;

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
        Logger::log("INFO", "Checking processes", serde_json::json!({
            "starting_with": starting_with,
        }));

        if let Err(err) = proc_dir.find_processes(&mut pids, &starting_with) {
            Logger::log("ERROR", "Error finding processes", serde_json::json!({
                "error": err.to_string(),
            }));
        }

        for &pid in pids.iter().filter(|&&pid| pid > 0) {
            let usage = checker.get_memory(pid)?;
            Logger::log("INFO", "Memory usage", serde_json::json!({
                "pid": pid,
                "usage_mb": bytes_to_megabytes(usage),
                "threshold_mb": bytes_to_megabytes(memory_threshold),
            }));

            if usage > memory_threshold {
                Logger::log("WARN", "Memory usage exeeded threshold, killing", serde_json::json!({
                    "pid": pid,
                    "usage_mb": bytes_to_megabytes(usage),
                    "threshold_mb": bytes_to_megabytes(memory_threshold),
                    "signal": signal,
                }));
                unsafe { libc::kill(pid as libc::pid_t, signal) };
            }
        }
        thread::sleep(sleep_duration);
    }
}

fn bytes_to_megabytes(bytes: u64) -> u64 {
    bytes / 1024 / 1024
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let process_name = cli.name;
    let max_memory: u64 = cli.max_memory as u64 * 1024 * 1024;
    let interval = cli.interval;
    let signal = cli.signal;

    Logger::log("WARN", "Starting memory-monitor", serde_json::json!({
        "process_name": process_name,
        "max_memory_mb": bytes_to_megabytes(max_memory),
        "interval": interval,
        "signal": signal
    }));

    monitor_processes(&process_name, max_memory, interval, &signal)
}
