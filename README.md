# memory-monitor

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
