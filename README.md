# Marble Factory

A Rust learning project for a small simulated automated factory. The project is
being built in small, testable increments so its domain boundaries and
communication patterns stay understandable as it grows.

## Current status

Chunk 1, *Workspace and domain vocabulary*, is in progress. The workspace is
healthy and contains two crates:

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

## Development conventions

- Rust source is formatted with the repository's `rustfmt.toml` configuration.
- VS Code settings format Rust on save, use rust-analyzer, display a 100-column
  ruler, and enable Clippy checks.
- The detailed implementation roadmap and decision record live in
  [`plans/README_marble_factory_chunk1.md`](plans/README_marble_factory_chunk1.md).

## Near-term scope

Chunk 1 will add strongly typed IDs, basic device descriptions, a minimal event
envelope, and a small `Factory` API that emits `PartCreated` events. It will not
add async runtime, networking, serialization, UI, or persistence dependencies.
