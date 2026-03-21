# ADR 003: RT-Safe Audio Graph with Double-Buffered Plan Swap

## Status
Accepted

## Context
Audio graphs must be modified (add/remove nodes) from the UI thread while the RT audio thread is processing. Mutex-based approaches risk priority inversion and audio glitches.

## Decision
Use a **double-buffered plan swap** pattern: the RT thread reads `current_plan` without locks; the non-RT thread places new plans in `pending_plan` behind a Mutex; the RT thread picks up pending plans via `try_lock()` (non-blocking).

## Rationale
- RT thread never blocks — `try_lock()` returns immediately on contention
- On mutex contention, RT thread continues with stale plan (no silence, no glitch)
- Lock-free metering (`AtomicU32` for peak levels) avoids all mutex usage on the RT path
- `GraphSwapHandle` provides a clean API for the non-RT thread

## Consequences
- Plan changes may be delayed by one buffer cycle (acceptable latency)
- Poisoned mutex is recovered gracefully (logged, plan installed)
- Graph compilation (topological sort) happens on non-RT thread
- `AudioNode` trait requires `Send` but not `Sync` — nodes are owned by the RT thread after swap
