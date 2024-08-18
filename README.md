# memory-monitor

## Motivation

When monitoring applications that might use _all_ your memory, your memory monitor may end up unable to kill
the process if the monitor itself requires memory allocations to do so. `memory-monitor` does no memory allocations
after startup.

`memory-monitor` is intended to be used as a companion process for a service running under systemd. systemd has
the ability to set memory limits on a service, but it only results in `oomkiller` sending a `SIGKILL`. With
`memory-monitor`, you can notice that your process is using too much memory and ask it to gracefully restart. If
the process receives the signal but doesn't stop, `memory-monitor` will continue periodically sending the signal
but will not take further action. Use systemd to hard-kill unresponsive processes.

## Usage

This tool monitors a process or processes by prefix, checking its memory usage at regular intervals. If the memory
usage exceeds a defined threshold, the tool sends a specified signal to the process.

### Command-Line Arguments

```bash
memory-monitor [OPTIONS] <name>
```

### Arguments

- **name**: The name of the process(es) to monitor. **This is a required argument.**

### Options

- **`-m, --max-memory <max_memory>`**: The maximum memory usage threshold in MB. If the process exceeds this memory usage, the specified signal will be sent. **This is a required option.**
- **`-i, --interval <interval>`**: The polling interval in seconds. This defines how frequently the tool checks the memory usage of the process. The default value is `2` seconds.
- **`-s, --signal <signal>`**: The signal to send to the process when the memory threshold is exceeded. The default value is `SIGTERM`.

### Examples

Monitor a process named `example-process` with a memory limit of 500 MB, checking every 3 seconds, and sending
`SIGUSR1` if the threshold is exceeded:

```bash
memory-monitor -m 500 -i 3 -s SIGUSR1 example-process
```

Monitor a process named `my-app` with a memory limit of 100 MB, using the default interval and signal:

```bash
memory-monitor -m 100 my-app
```


## Debugging Memory Allocations

You'll first need to codesign your binary with the right entitlements:

```
$ codesign -s - -v -f --entitlements debug.plist ./target/debug/memory-monitor
```

Then, create a trace:

```
$ xctrace record --template Allocations --launch -- ./target/debug/memory-monitor --max-memory 100 tyson_api
```

Then open the trace with Instruments.app:

```
$ open Launch*
```
