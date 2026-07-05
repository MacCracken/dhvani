# ADR 003: RT-Safe Audio Graph with Double-Buffered Plan Swap

## Status
**Accepted** — still holds for the Cyrius graph port, with a mechanism update
(no threads/Mutex). See below. The historical Rust-era decision is kept below as
a record.

## Update (Cyrius port)

The double-buffered plan-swap design survives the port intact. Nodes are now
fn-ptr based: a `DhGraphNode` holds a `process_fp` fn-ptr + opaque `state`
pointer, and `process()` dispatches via `fncall3(process_fp, state, inputs,
output)` (Rust's `AudioNode` trait object → fn-ptr; see `src/graph.cyr`). The RT
`GraphProcessor` still reads a `current_plan` and picks up new plans from a
`pending` cell handed over by a `GraphSwapHandle` (`swap`/`clone`).

Two mechanism details differ from the Rust decision below:

- The `Arc<Mutex<Option<plan>>>` pending cell became a shared **length-1 vec**
  cell. Cyrius has no threads in this port, so there is no Mutex, no
  `try_lock()`, and no contention/poisoning path — the swap is a single-slot
  take/replace.
- `AtomicU32` metering and the `Send`/`Sync` trait bounds are Rust-specific and
  do not carry over. `process_parallel` (`#[cfg(feature="parallel")]`, rayon) was
  dropped (no threads). Topological compilation still happens off the process
  loop.

---

## Historical (Rust 1.x)

### Context
Audio graphs must be modified (add/remove nodes) from the UI thread while the RT audio thread is processing. Mutex-based approaches risk priority inversion and audio glitches.

### Decision
Use a **double-buffered plan swap** pattern: the RT thread reads `current_plan` without locks; the non-RT thread places new plans in `pending_plan` behind a Mutex; the RT thread picks up pending plans via `try_lock()` (non-blocking).

### Rationale
- RT thread never blocks — `try_lock()` returns immediately on contention
- On mutex contention, RT thread continues with stale plan (no silence, no glitch)
- Lock-free metering (`AtomicU32` for peak levels) avoids all mutex usage on the RT path
- `GraphSwapHandle` provides a clean API for the non-RT thread

### Consequences
- Plan changes may be delayed by one buffer cycle (acceptable latency)
- Poisoned mutex is recovered gracefully (logged, plan installed)
- Graph compilation (topological sort) happens on non-RT thread
- `AudioNode` trait requires `Send` but not `Sync` — nodes are owned by the RT thread after swap
