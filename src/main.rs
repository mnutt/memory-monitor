use clap::Parser;
use mockall::automock;
use std::error::Error;
use std::io;
use std::thread;
use std::time::Duration;
use log::{info, warn, error};
use env_logger;
use env_logger::Env;

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

#[automock]
trait ProcDir {
    fn open() -> Result<Self, io::Error>
    where
        Self: Sized;
    fn find_processes(&mut self, pids: &mut Vec<i32>, starting_with: &str) -> io::Result<()>;
}

#[automock]
trait MemoryChecker {
    fn new() -> Self
    where
        Self: Sized;
    fn get_memory(&mut self, pid: i32) -> Result<u64, String>;
    fn kill(&self, pid: libc::pid_t, signal: i32);
}

#[cfg(target_os = "macos")]
use mac::{MacMemoryChecker as MemoryCheckerImpl, MacProcDir as ProcDirImpl};

#[cfg(target_os = "linux")]
use linux::{LinuxMemoryChecker as MemoryCheckerImpl, LinuxProcDir as ProcDirImpl};

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

fn monitor_processes<P: ProcDir, M: MemoryChecker>(
    proc_dir: &mut P,
    checker: &mut M,
    starting_with: &str,
    memory_threshold: u64,
    interval: u16,
    signal: &str,
    single: bool,
) -> Result<(), Box<dyn Error>> {
    let mut pids: Vec<i32> = Vec::with_capacity(PID_COUNT_MAX);

    let sleep_duration = Duration::from_secs(interval as u64);
    let signal_code = signal_from_string(signal).unwrap_or(libc::SIGTERM);

    loop {
        info!("Checking processes starting with {}", starting_with);

        if let Err(err) = proc_dir.find_processes(&mut pids, &starting_with) {
            error!("Error finding processes: {}", err);
        }

        for &pid in pids.iter() {
            info!("  Checking memory usage for pid {}", pid);
            let usage = checker.get_memory(pid)?;
            info!(
                "  Process pid: {} memory: {} MB",
                pid,
                bytes_to_megabytes(usage)
            );
            if usage > memory_threshold {
                warn!(
                    "  Killing pid {} with memory: {} MB, which is over the threshold of {} MB",
                    pid,
                    bytes_to_megabytes(usage),
                    bytes_to_megabytes(memory_threshold)
                );
                checker.kill(pid as libc::pid_t, signal_code);
                warn!("  Killed pid {} with signal {}", pid, signal);
            }
        }

        info!("Checked {} processes", pids.len());

        if single {
            return Ok(());
        } else {
            thread::sleep(sleep_duration);
        }
    }
}

fn bytes_to_megabytes(bytes: u64) -> u64 {
    bytes / 1024 / 1024
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    warn!("Starting memory monitor");

    let cli = Cli::parse();

    let process_name = cli.name;
    let max_memory: u64 = cli.max_memory as u64 * 1024 * 1024;
    let interval = cli.interval;
    let signal = cli.signal;

    warn!(
        "Monitoring processes starting with {} that use more than {} MB of memory, polling every {} seconds, sending signal {}",
        process_name,
        bytes_to_megabytes(max_memory),
        interval,
        signal
    );

    // These perform all of the memory allocations up-front
    let mut proc_dir = ProcDirImpl::open()?;
    let mut checker = MemoryCheckerImpl::new();

    monitor_processes(
        &mut proc_dir,
        &mut checker,
        &process_name,
        max_memory,
        interval,
        &signal,
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use mockall::predicate::*;

    #[test]
    fn test_signal_from_string() {
        assert_eq!(signal_from_string("SIGUSR1"), Some(libc::SIGUSR1));
        assert_eq!(signal_from_string("SIGUSR2"), Some(libc::SIGUSR2));
        assert_eq!(signal_from_string("SIGTERM"), Some(libc::SIGTERM));
        assert_eq!(signal_from_string("SIGKILL"), Some(libc::SIGKILL));
        assert_eq!(signal_from_string("UNKNOWN"), None);
    }

    #[test]
    fn test_bytes_to_megabytes() {
        assert_eq!(bytes_to_megabytes(1048576), 1); // 1 MB
        assert_eq!(bytes_to_megabytes(2097152), 2); // 2 MB
        assert_eq!(bytes_to_megabytes(0), 0); // 0 MB
    }

    #[test]
    fn test_cli_parsing() {
        let args = vec![
            "test",
            "my_process",
            "--max-memory",
            "100",
            "--interval",
            "5",
            "--signal",
            "SIGKILL",
        ];
        let cli = Cli::parse_from(args);

        assert_eq!(cli.name, "my_process");
        assert_eq!(cli.max_memory, 100);
        assert_eq!(cli.interval, 5);
        assert_eq!(cli.signal, "SIGKILL");
    }

    #[test]
    fn test_cli_default_values() {
        let args = vec!["test", "my_process", "--max-memory", "100"];
        let cli = Cli::parse_from(args);

        assert_eq!(cli.name, "my_process");
        assert_eq!(cli.max_memory, 100);
        assert_eq!(cli.interval, 2); // Default value
        assert_eq!(cli.signal, "SIGTERM"); // Default value
    }

    #[test]
    fn test_monitor_processes_no_processes() {
        let mut mock_proc_dir = MockProcDir::new();
        let mut mock_memory_checker = MockMemoryChecker::default();

        // Set up the mock to return no processes
        mock_proc_dir
            .expect_find_processes()
            .withf(|_, _| true)
            .returning(|pids, _| {
                pids.clear();
                Ok(())
            });

        let result = monitor_processes(
            &mut mock_proc_dir,
            &mut mock_memory_checker,
            "non_existent_process",
            1024 * 1024,
            2,
            "SIGTERM",
            true,
        );

        assert!(
            result.is_ok(),
            "Expected monitor_processes to handle no processes gracefully"
        );
    }

    #[test]
    fn test_monitor_processes_with_none_exceeding_threshold() {
        let mut mock_proc_dir = MockProcDir::new();
        let mut mock_memory_checker = MockMemoryChecker::default();

        // Set up the mock to return a single process with memory usage under the threshold
        mock_proc_dir
            .expect_find_processes()
            .withf(|_, _| true)
            .returning(|pids, _| {
                pids.clear();
                pids.push(123);
                Ok(())
            });

        mock_memory_checker
            .expect_get_memory()
            .with(eq(123))
            .returning(|_| Ok(2 * 1024 * 1024));

        mock_memory_checker.expect_kill().times(0);

        let result = monitor_processes(
            &mut mock_proc_dir,
            &mut mock_memory_checker,
            "non_existent_process",
            4 * 1024 * 1024,
            2,
            "SIGTERM",
            true,
        );

        assert!(
            result.is_ok(),
            "Expected monitor_processes to handle no processes exceeding the threshold gracefully"
        );
    }

    #[test]
    fn test_monitor_processes_with_single_process_exceeding_threshold() {
        let mut mock_proc_dir = MockProcDir::new();
        let mut mock_memory_checker = MockMemoryChecker::default();

        // Set up the mock to return a single process with memory usage over the threshold
        mock_proc_dir
            .expect_find_processes()
            .withf(|_, _| true)
            .returning(|pids, _| {
                pids.clear();
                pids.push(123);
                Ok(())
            });

        mock_memory_checker
            .expect_get_memory()
            .with(eq(123))
            .returning(|_| Ok(5 * 1024 * 1024));

        mock_memory_checker
            .expect_kill()
            .with(eq(123), eq(libc::SIGTERM))
            .times(1)
            .returning(|_, _| ());

        let result = monitor_processes(
            &mut mock_proc_dir,
            &mut mock_memory_checker,
            "non_existent_process",
            4 * 1024 * 1024,
            2,
            "SIGTERM",
            true,
        );

        assert!(result.is_ok(), "Expected monitor_processes to handle a single process exceeding the threshold gracefully");
    }
}
