# memory-monitor

## Motivation

When monitoring applications that might use _all_ your memory, your memory monitor may end up unable to kill
the process if the monitor itself requires memory allocations to do so. `memory-monitor` does no memory allocations
after startup.

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
