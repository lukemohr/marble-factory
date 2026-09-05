# Marble Factory

A Rust learning project for a small simulated automated factory. The project is
being built in small, testable increments so its domain boundaries and
communication patterns stay understandable as it grows.

## Current status

Chunk 2, *Deterministic simulation time*, is in progress. Chunk 1, *Workspace
and domain vocabulary*, is complete. The workspace contains two crates:

```text
protocol  <-  simulator
```

Run the complete test suite with:

```bash
cargo test --workspace
```

## Architectural decisions

| Decision | Choice | Why |
| --- | --- | --- |
| Initial project layout | Cargo workspace | Makes the protocol boundary explicit before more components are added. |
| Dependency direction | `simulator` depends on `protocol` | Shared boundary data must not depend on simulation behavior. |
| External dependencies in Chunk 1 | None | Add a dependency only when a current requirement justifies it. |
| Identifiers | Private `u64` newtypes | `PartId` and `DeviceId` cannot be mixed accidentally; values remain deterministic. |
| ID traits | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash` | Supports tests, inexpensive passing, equality, and future map keys without implying meaningful ordering. |
| Protocol organization | Schema-focused modules in `protocol` | `ids`, `devices`, and later `events` organize boundary data without introducing a redundant crate. |
| Device categories | `DeviceKind` in `protocol` | Device category is shared descriptive data; device behavior remains outside the protocol. |
| Device descriptions | Public `DeviceDescriptor` fields | It has no invariants today. Revisit private fields and a constructor if validation, kind-specific configuration, or immutability requirements arise. |
| Event representation | One `Event` enum | The near-term system has one event stream; consumers can exhaustively pattern-match its variants. |
| Event metadata | Minimal `FactoryEvent` envelope | A sequence number orders every event; timestamps wait until simulation time has an explicit meaning. |
| ID and sequence allocation | `Factory`, starting at `0` | The factory owns part creation, so it owns its counters; a separate allocator has no current use. |
| Empty factory construction | `Factory::new()` and `Default` | An empty zero-based factory has one clear default state. |
| Simulator visibility | Binary-only internal `Factory` | Expose a simulator library only when another component actually needs in-process access. |
| Simulation-time semantics | Elapsed, caller-controlled time since simulation start | It begins at zero and advances monotonically only through explicit updates; future wall-clock scheduling remains outside the deterministic factory. |
| Simulation-time representation | `std::time::Duration` | Standard-library elapsed-time value with non-negative semantics; no fixed precision or external dependency is needed yet. |
| Event timestamps | `FactoryEvent.sim_time` in the protocol | An event records the factory's elapsed simulation time when it occurred—not wall-clock time or an event duration. Sequence remains the total order for same-time events. |
| Clock storage | Private `Factory::sim_time: Duration` field | The factory is the only clock owner today; extract a `SimulationClock` only when it gains independent behavior such as scheduling or pause/rate control. |
| Time advancement | Caller-supplied delta to `Factory::update(dt)` | Callers control elapsed time explicitly, making replay direct and preventing backward progression by API shape. |
| Update event collection | `update()` returns `Vec<FactoryEvent>` | State changes and their time-driven effects are returned together; revisit an internal queue or sink only when a current need appears. |
| Mutation event shapes | Direct actions return one event; `update()` returns many-or-none | `create_part()` always creates one part, whereas a time interval can produce zero or more effects. |
| Zero-duration update | Valid no-op | `update(Duration::ZERO)` preserves time and produces no events until immediate time-driven behavior exists. |
| Simulation-time overflow | Checked addition, panic on overflow | An overflow is an invalid simulation run; failing loudly preserves time semantics without premature error infrastructure. |
| Clock observation | Read-only `Factory::sim_time()` query | Current time is intentional observable behavior while its storage stays private. |
| Tick events | None in the domain-event stream | Update cycles are execution details; add a purpose-built diagnostics or control mechanism only when a real consumer needs step boundaries. |
| Event ordering | `sim_time` for occurrence time; `sequence` for total order | Same-time events are valid; sequence is strictly increasing and local to one factory simulation, so stream order wins when exact ordering matters. |
| Simulator organization | Internal `factory` module | The factory and its tests are now a cohesive unit; `main.rs` stays reserved for a later runtime loop. |

## Development conventions

- Rust source is formatted with the repository's `rustfmt.toml` configuration.
- VS Code settings format Rust on save, use rust-analyzer, display a 100-column
  ruler, and enable Clippy checks.
- The detailed implementation roadmap and decision record live in
  [`plans/README_marble_factory_chunk1.md`](plans/README_marble_factory_chunk1.md).

## Near-term scope

Chunk 2 introduces an explicit, caller-controlled simulation clock and a
deterministic update loop. Async runtime, networking, serialization, UI, and
persistence remain out of scope.
