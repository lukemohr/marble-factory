# Marble Factory

A Rust learning project focused on software architecture for a small simulated automated factory.

The long-term goal is to build a system with:

- a simulated factory floor,
- independently running controllers,
- explicit command/response handshakes,
- event-driven data transfer,
- real-time state and feedback,
- a visual browser-based HMI,
- and eventually a message bus such as NATS.

This project is intentionally being built in small, testable chunks. The purpose is not just to get working code, but to practice making and defending architectural decisions.

---

# How to use this repository

I am building this project with help from a Codex agent in VS Code.

The agent should **not simply implement the entire design for me**.

Instead, it should:

1. explain the next small step,
2. identify architectural choices when they matter,
3. ask me to choose between reasonable alternatives,
4. explain the tradeoffs,
5. let me write or approve the implementation,
6. review what exists before moving on,
7. avoid introducing abstractions or dependencies before they are useful.

Prefer small, testable increments over large code dumps.

When there are multiple reasonable designs, do not silently choose one. Present the decision explicitly.

---

# Project direction

The eventual system may look roughly like this:

```text
┌───────────────────────────────┐
│          Browser HMI          │
│ factory view / controls /     │
│ alarms / event inspector      │
└───────────────┬───────────────┘
                │ WebSocket
                ▼
┌───────────────────────────────┐
│          HMI Server           │
└───────────────┬───────────────┘
                │
                │ NATS
                │
      ┌─────────┼─────────┐
      ▼         ▼         ▼
┌──────────┐ ┌─────────┐ ┌────────────┐
│Controller│ │Telemetry│ │ Diagnostics│
└────┬─────┘ └─────────┘ └────────────┘
     │
     │ commands / events
     ▼
┌───────────────────────────────┐
│       Factory Simulator       │
│                               │
│ conveyor → sensor → diverter  │
│                ↘              │
│                 bins          │
└───────────────────────────────┘
```

We are **not** starting with this full architecture.

The first version should run locally and remain deliberately simple so the protocol and domain boundaries can be designed before networking is introduced.

---

# Development roadmap

The planned progression is approximately:

1. Workspace and domain vocabulary
2. Deterministic simulation clock
3. Parts moving through conveyor segments
4. Sensors and factory events
5. Device command protocol
6. Diverter controller and command handshake
7. Independent controller task
8. Desired state vs. observed state
9. Browser visualization
10. Replace in-process communication with NATS
11. Faults, timeouts, retries, and recovery
12. Network chaos / fault injection
13. Persistence, recipes, inspection, scheduling, metrics, and other extensions

This README currently focuses on **Chunk 1**.

## Progress record

This section records decisions and implementation progress as Chunk 1 proceeds.

| Step | Status | Decision / outcome |
| --- | --- | --- |
| 0 — Verify repository | Complete | The repository root is `marble-factory`; no Rust workspace existed and no project files were overwritten. |
| 1 — Initialize workspace | Complete | Chose workspace-first: a root Cargo workspace with `protocol` and `simulator` members. |
| 2 — Create workspace | Complete | `simulator` has a local dependency on `protocol`; no external dependencies were added; `cargo test --workspace` passes. |
| 3 — ID representation | Complete | Chose private `u64` newtypes over UUIDs and raw aliases: `PartId` and `DeviceId` have `const` constructors. |
| 4 — ID traits | Complete | Derived `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, and `Hash`; intentionally omitted ordering traits. |
| 5 — Device kinds | Complete | Kept boundary data in the `protocol` schema crate, organized with `ids` and `devices` modules. Added data-only `DeviceKind`. |
| 6 — Device descriptions | Complete | Added public `DeviceDescriptor { id, kind }` data. Reconsider private fields and a constructor only if real descriptor invariants emerge. |
| 7 — First event | Complete | Added `Event::PartCreated { part_id }` to a shared `events` module. Chose one enum because events will share a stream. |
| 8–9 — Event envelope and time | Complete | Added public `FactoryEvent { sequence, event }`. Deliberately omitted timestamps until Chunk 2 defines simulation time. |

Supporting project files added during setup: root `.gitignore`, `README.md`,
`rustfmt.toml`, and VS Code workspace settings.

---

# Chunk 1 — Workspace and domain vocabulary

## Goal

Create the smallest useful foundation for the project.

At the end of Chunk 1, the repository should contain a Rust workspace with:

```text
marble-factory/
├── Cargo.toml
└── crates/
    ├── protocol/
    └── simulator/
```

The intent is to establish an important dependency direction:

```text
protocol
   ▲
   │
simulator
```

The `protocol` crate defines information that may cross component boundaries.

The `simulator` crate contains internal simulation behavior.

The protocol crate must not depend on the simulator crate.

Eventually other components should also depend on the shared protocol:

```text
          protocol
        ▲    ▲    ▲
       /     |     \
simulator controller hmi-server
```

---

# Step 0 — Verify the empty repository

Before creating anything, inspect the repository.

Confirm that:

- this directory is the Git repository root,
- `.git/` already exists,
- there is not already a Rust workspace here,
- no existing files would be overwritten.

Do not create a nested `marble-factory/marble-factory` directory.

The Rust workspace should live directly inside the cloned repository.

---

# Step 1 — Decide how to initialize the workspace

Before running commands, stop and discuss this decision:

## Architectural decision: workspace-first or application-first?

Two reasonable approaches are:

### Option A — workspace-first

Create the root `Cargo.toml` manually and then create member crates.

Example target structure:

```text
Cargo.toml
crates/protocol
crates/simulator
```

Advantages:

- makes the architectural separation explicit immediately,
- scales naturally as more services are added,
- avoids needing to reorganize a single crate later.

### Option B — application-first

Start with one normal Rust package and split it later.

Advantages:

- less setup,
- easier for very small projects.

For this project, workspace-first is probably the stronger choice, but **ask me to confirm before proceeding**.

---

# Step 2 — Create the workspace

If workspace-first is chosen, create a root `Cargo.toml` similar to:

```toml
[workspace]
resolver = "3"
members = [
    "crates/protocol",
    "crates/simulator",
]
```

Then create:

```bash
cargo new crates/protocol --lib
cargo new crates/simulator --bin
```

Add this dependency to `crates/simulator/Cargo.toml`:

```toml
[dependencies]
protocol = { path = "../protocol" }
```

At this stage, use no external dependencies unless there is a compelling reason.

In particular, do **not** add these yet:

- Tokio
- serde
- UUID
- NATS
- tracing
- web frameworks

We will introduce dependencies when the problem requires them.

After setup, run:

```bash
cargo test --workspace
```

and make sure the workspace builds before continuing.

---

# Step 3 — Define strongly typed identities

We need identifiers for things in the factory.

Initial candidates:

```rust
pub struct PartId(pub u64);
pub struct DeviceId(pub u32);
```

Before implementing them, stop and discuss:

## Architectural decision: raw primitive IDs or newtypes?

Compare:

```rust
type PartId = u64;
type DeviceId = u64;
```

with:

```rust
struct PartId(u64);
struct DeviceId(u64);
```

Questions to consider:

- Can a `PartId` accidentally be passed where a `DeviceId` is expected?
- Does the extra type safety justify slightly more boilerplate?
- Should IDs expose their inner value publicly?
- If not, what constructors/accessors should exist?
- Do we expect these IDs to cross process boundaries later?

The preferred direction for this project is likely newtypes, but I want to make the choice consciously.

---

# Step 4 — Choose useful traits for IDs

Do not automatically derive every trait.

For each ID type, consider whether we need:

```rust
Debug
Clone
Copy
PartialEq
Eq
Hash
PartialOrd
Ord
```

Prompt me to explain which ones I think are appropriate.

Useful questions:

- Will IDs be keys in a `HashMap`?
- Should copying an ID be cheap and unsurprising?
- Do IDs have meaningful ordering, or would deriving `Ord` imply semantics we do not want?
- Will tests need to compare IDs directly?

After the discussion, implement only the traits we currently have a reason for.

---

# Step 5 — Represent device kinds

The first factory may eventually contain:

- conveyors,
- sensors,
- diverters,
- bins.

A starting point might be:

```rust
pub enum DeviceKind {
    Conveyor,
    Sensor,
    Diverter,
    Bin,
}
```

Before implementing, discuss:

## Architectural decision: what belongs in the protocol?

Ask:

- Is `DeviceKind` something other components need to know?
- Is it protocol data, simulator implementation detail, or both?
- Are we defining behavior here, or only describing identity/category?
- Would adding a new simulator-only device force unrelated components to change?

The goal is to avoid turning `protocol` into a dumping ground for every domain type.

---

# Step 6 — Represent device descriptions

We want to distinguish:

```text
Device identity:
    Sensor #4

Device category:
    Sensor
```

A possible representation is:

```rust
pub struct DeviceDescriptor {
    pub id: DeviceId,
    pub kind: DeviceKind,
}
```

Before using this exact design, stop and discuss:

## Architectural decision: descriptor vs. device object

Questions:

- Does this type describe a device, or represent the actual device?
- Should the protocol crate ever contain device behavior?
- Would the name `Device` imply more ownership or behavior than this type really has?
- Is `DeviceDescriptor` clearer?
- Should fields be public, or should construction be controlled?

The protocol crate should contain data contracts, not simulator implementations.

---

# Step 7 — Introduce the first event

The system will eventually communicate with both:

- events,
- commands.

For Chunk 1, implement only one event:

```text
PartCreated
```

A possible payload enum:

```rust
pub enum Event {
    PartCreated {
        part_id: PartId,
    },
}
```

Before implementation, discuss:

## Architectural decision: enum events or separate event structs?

Compare:

```rust
enum Event {
    PartCreated { part_id: PartId },
}
```

with separate types such as:

```rust
struct PartCreated {
    part_id: PartId,
}
```

Questions:

- How will consumers pattern-match events?
- Will all events travel over the same channel eventually?
- Would an enum make serialization easier later?
- Could a very large event enum become unwieldy?

Choose the simplest design that supports the near-term system.

---

# Step 8 — Add an event envelope

Instead of sending only:

```rust
Event::PartCreated { ... }
```

introduce an outer message:

```rust
pub struct FactoryEvent {
    pub sequence: u64,
    pub event: Event,
}
```

Before committing to this, discuss:

## Architectural decision: payload vs. envelope

Why might an envelope be useful?

Potential future envelope fields include:

- timestamp,
- source device,
- correlation ID,
- trace ID,
- schema version.

Questions:

- Which information applies to every event?
- Which information belongs only to specific event payloads?
- Does putting metadata in an envelope reduce duplication?
- Is it premature abstraction, or is the boundary already clear enough to justify it?

For Chunk 1, the envelope should remain minimal.

---

# Step 9 — Deliberately do not add timestamps

Do not add timestamps yet.

Before continuing, discuss why.

Possible meanings of "event time" include:

- simulation time,
- wall-clock time,
- time the physical event occurred,
- time the event object was created,
- time the event was sent,
- time the event was received.

These are not necessarily the same thing.

Chunk 2 will introduce simulation time explicitly.

For now, event ordering is represented only by the sequence number.

---

# Step 10 — Build the first simulator state

Inside the simulator crate, create a small `Factory` type.

A possible starting point:

```rust
struct Factory {
    next_part_id: u64,
    next_event_sequence: u64,
}
```

Give it:

```rust
fn new() -> Self
```

and something equivalent to:

```rust
fn create_part(&mut self) -> FactoryEvent
```

Calling `create_part` should:

1. allocate a unique `PartId`,
2. advance the part ID counter,
3. allocate an event sequence,
4. advance the sequence counter,
5. return a `FactoryEvent`,
6. use an `Event::PartCreated` payload.

Before implementation, discuss:

## Architectural decision: where should ID allocation live?

Possible choices include:

- inside `Factory`,
- in a dedicated ID generator,
- externally,
- in the protocol layer.

Questions:

- Who owns part creation?
- Is ID generation currently a separate responsibility worth abstracting?
- Would extracting an allocator now improve anything?
- What would make us revisit this decision later?

Prefer the simplest ownership model that is clear today.

---

# Step 11 — Protect simulator internals

The outside world should not need direct access to:

```rust
next_part_id
next_event_sequence
```

Discuss:

## Architectural decision: state visibility

Questions:

- Which state is implementation detail?
- What information should callers learn from emitted events instead?
- Do tests require private state to become public?
- Can behavior be tested through public APIs instead?

Avoid making fields public merely to simplify tests.

---

# Step 12 — Write tests

Chunk 1 should end with tests, not a large demo application.

At minimum, test these behaviors.

## Test A — first part ID

Creating the first part produces:

```text
PartId(0)
```

unless we intentionally choose a different starting convention.

Before writing the test, ask whether IDs should begin at `0` or `1`.

That is a small decision, but it should be explicit.

---

## Test B — unique part IDs

Creating several parts should produce distinct IDs.

The test should check behavior rather than internal counters.

---

## Test C — monotonically increasing event sequence numbers

For example:

```text
0
1
2
3
```

Again, test the public behavior.

---

# Step 13 — Review the public API

Before declaring Chunk 1 complete, inspect the public API of both crates.

Ask:

- Is anything public that does not need to be?
- Does the simulator expose implementation details?
- Does the protocol crate contain behavior that belongs elsewhere?
- Are names communicating intent?
- Is any abstraction present only because we imagine needing it someday?
- Are dependencies flowing in the intended direction?

Do not continue to Chunk 2 until this review is complete.

---

# Chunk 1 definition of done

The approximate public vocabulary should be:

```text
protocol
├── PartId
├── DeviceId
├── DeviceKind
├── DeviceDescriptor
├── FactoryEvent
└── Event
```

and the simulator should contain something conceptually like:

```text
Factory
└── create_part()
```

The exact names and visibility may differ if we make better decisions during implementation.

The following command must succeed:

```bash
cargo test --workspace
```

Chunk 1 should still have:

- no Tokio,
- no NATS,
- no serde,
- no WebSocket,
- no UI,
- no physics engine,
- no database.

---

# Guidance for the Codex agent

While working through Chunk 1:

- do not skip directly to the final implementation,
- do not implement future chunks,
- do not add dependencies "for later,"
- do not create generic abstractions without a current use case,
- do not silently decide architectural questions.

At each meaningful choice:

1. state the decision,
2. give 2–3 realistic options,
3. explain the tradeoffs concisely,
4. recommend one if appropriate,
5. ask me to choose,
6. then implement or guide the implementation.

When reviewing my code:

- prioritize correctness,
- then API boundaries,
- then ownership/responsibility,
- then simplicity,
- then style.

If the current implementation is reasonable, say so instead of inventing refactors.

The goal of this project is to learn to recognize **when architecture matters**, not to maximize abstraction.

---

# What comes next

After Chunk 1 is reviewed and complete, the next milestone is:

## Chunk 2 — deterministic simulation time

That chunk will introduce:

- a simulation clock,
- a deterministic update loop,
- explicit elapsed time,
- and the first important question around how state mutation and event production should interact.

Do not begin Chunk 2 until Chunk 1 has passed its tests and architectural review.
