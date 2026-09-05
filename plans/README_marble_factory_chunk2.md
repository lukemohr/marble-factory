# Chunk 2 — Deterministic Simulation Time

This document continues the Marble Factory project after completion of Chunk 1.

Chunk 1 established:

- a Rust workspace,
- a `protocol` crate for schema-only boundary data,
- a `simulator` crate for internal factory behavior,
- strongly typed `PartId` and `DeviceId` newtypes,
- `DeviceKind` and `DeviceDescriptor`,
- a shared `Event` enum,
- a `FactoryEvent { sequence, event }` envelope,
- zero-based part and event sequencing,
- and a deliberately minimal `Factory` implementation with public-behavior tests.

Chunk 2 should preserve the same development philosophy:

- make small, testable changes,
- introduce abstractions only when a current requirement justifies them,
- surface architectural choices explicitly,
- prefer deterministic behavior over convenience,
- and do not introduce async/networking concerns yet.

The purpose of Chunk 2 is to introduce **simulation time** and a **deterministic update loop** without yet adding conveyor motion, sensors, Tokio, NATS, or wall-clock scheduling.

---

# Chunk 2 goal

At the end of this chunk, the simulator should be able to:

1. maintain an explicit simulation clock,
2. advance simulation time deterministically,
3. execute a simulation update/tick,
4. make the relationship between state mutation and emitted events explicit,
5. attach simulation time to events in a well-defined way,
6. produce identical results when given identical initial state and identical time-step inputs.

The system should **not** depend on real elapsed wall-clock time.

A caller should control time progression explicitly.

Conceptually:

```text
initial state
    │
    │ advance by Δt
    ▼
Factory::update(...)
    │
    ├── mutate simulation state
    │
    └── produce zero or more events
    ▼
new deterministic state
```

The core idea is:

> Given the same starting state and the same sequence of update durations, the simulator must produce the same state and event sequence.

---

# Progress record

As Chunk 2 proceeds, add decisions here.

| Step | Status | Decision / outcome |
| --- | --- | --- |
| 0 — Review Chunk 1 baseline | Complete | `protocol` remains schema-only; the binary `simulator` owns an internal `Factory` with private counters; no time concepts or dependencies exist; workspace tests pass. |
| 1 — Define simulation-time semantics | Complete | Simulation time is elapsed time since factory start: zero-based, caller-controlled, monotonic, and conceptually continuous. Real-time scheduling may later supply deltas externally but will not drive the factory from wall time. |
| 2 — Choose time representation | Complete | Chose `std::time::Duration`: it expresses non-negative elapsed time without fixing units or adding a dependency. |
| 3 — Decide protocol vs simulator ownership | Complete | `FactoryEvent` will expose the same `Duration` used by the factory because event time is shared contract data; no second representation or conversion is needed. |
| 4 — Add simulation time to events | Complete | Added `FactoryEvent.sim_time: Duration`. It means elapsed simulation time at the event occurrence, not wall-clock time or an event duration; multiple events may share it and use sequence for total order. |
| 5 — Introduce simulation clock state | Complete | `Factory` owns a direct private `sim_time: Duration` field, initialized to zero. A dedicated clock type is deferred until it has behavior beyond storing the current time. |
| 6 — Define update API | Complete | Chose caller-supplied delta time: `Factory::update(dt)`. This keeps time progression explicit, replayable, and forward-only by construction. |
| 7 — Decide event collection model | Complete | `update()` will return `Vec<FactoryEvent>`. This is intentionally revisitable if a concrete need for a queue or streaming sink emerges. |
| 8 — Non-update actions | Complete | `create_part()` remains `-> FactoryEvent` while `update()` returns `Vec<FactoryEvent>`. The difference reflects guaranteed-one versus zero-or-more events, not inconsistent protocol semantics. |
| 9 — Zero-duration behavior | Complete | `update(Duration::ZERO)` is a valid no-op: it preserves current simulation time and returns no events while no immediate work exists. |
| 10 — Invalid/overflow behavior | Complete | `Duration` excludes negative deltas. Clock addition is checked and panics clearly on overflow; saturation would silently corrupt time semantics, and `Result` is premature here. |
| 11 — Observe current time | Complete | Added the intentional read-only `Factory::sim_time()` query; clock storage remains private. |
| 12 — Timestamp direct actions | Complete | `create_part()` emits at the factory's current simulation time. Multiple creations between updates correctly share a timestamp while sequence increases. |
| 13 — First deterministic update | Complete | `update(dt)` uses checked addition to advance time and returns an empty `Vec` until time-driven behavior is added. |
| 14 — Tick events | Pending decision | |
| 15 — Deterministic tests | Complete | Tests cover initial time, deltas, zero updates, timestamps, same-time ordering, overflow, deterministic replay, and partitioned advancement with no time-driven state. |
| 16 — Public API review | Pending final review | |

---

# Step 0 — Review the current baseline

Before editing code, inspect the current implementation.

Confirm:

- `protocol` still contains schema/data types only,
- `simulator` still owns `Factory`,
- `Factory` still owns private part-ID and event-sequence counters,
- no timestamps or clocks already exist,
- `cargo test --workspace` passes,
- no external dependencies were added since Chunk 1.

Do not reorganize working Chunk 1 code unless Chunk 2 gives a concrete reason.

Before continuing, summarize the current relevant code structure.

---

# Step 1 — Define what "simulation time" means

Before choosing a Rust type, stop and define semantics.

## Architectural decision: what clock are we modeling?

Possible time concepts include:

### A. Simulation elapsed time

Time since the simulated factory started.

Example:

```text
0 ms
100 ms
200 ms
300 ms
```

### B. Wall-clock / calendar time

Example:

```text
2026-09-05T14:22:10.125-04:00
```

### C. Step number

Example:

```text
tick 0
tick 1
tick 2
tick 3
```

For this chunk, strongly consider using **simulation elapsed time** as the primary clock.

Discuss:

- Why is wall-clock time undesirable for deterministic tests?
- Is tick number sufficient if update durations may eventually vary?
- Should simulation time begin at zero?
- Is simulation time continuous in concept even if represented discretely?
- Will pausing the simulation advance simulation time?
- Should the simulation clock be monotonic by construction?

Record the decision before implementation.

---

# Step 2 — Choose the time representation

Do not immediately use `std::time::Instant`.

Discuss the available choices.

## Option A — `std::time::Duration`

Example:

```rust
Duration::from_millis(250)
```

Advantages:

- standard library type,
- clear elapsed-time semantics,
- prevents negative durations,
- useful arithmetic support,
- no external dependency.

Questions:

- Does `Duration` serialize cleanly later?
- Does using a standard-library type in `protocol` create future wire-format concerns?
- Is nanosecond precision meaningful for this factory?

## Option B — integer time units

Example:

```rust
pub struct SimTimeMs(u64);
```

Advantages:

- explicit wire representation,
- trivial comparison and serialization later,
- domain-specific semantics.

Questions:

- Are milliseconds sufficient?
- Does fixing units now create unnecessary constraints?
- Would a newtype avoid accidentally mixing durations and absolute simulation times?

## Option C — floating-point seconds

Example:

```rust
f64
```

Advantages:

- convenient for physics formulas.

Disadvantages to consider:

- equality and deterministic comparisons,
- accumulated floating-point error,
- unclear units.

Before implementing, ask me to choose.

A reasonable recommendation is likely either:

- `Duration`, or
- explicit integer/newtype simulation time.

Do not introduce an external time crate.

---

# Step 3 — Decide where simulation-time types belong

This is a boundary decision.

## Architectural decision: protocol or simulator?

Suppose we define a type like:

```rust
pub struct SimTime(...);
```

Where should it live?

### Option A — `protocol`

Rationale:

- emitted events may include simulation timestamps,
- future controllers/HMI may need to interpret them.

### Option B — `simulator`

Rationale:

- simulation time is initially an internal mechanism,
- avoid exposing concepts before consumers require them.

Discuss:

- Is simulation time part of the message contract once events include it?
- Could the simulator use one internal representation and expose another?
- Would that be useful now, or unnecessary conversion?
- Does the protocol crate remain "schema-only" if it owns a `SimTime` value type?

Prefer a single representation unless a concrete need justifies two.

Record the decision.

---

# Step 4 — Decide how event timestamps should work

Chunk 1 deliberately postponed timestamps.

Now define exactly what the timestamp means.

A candidate event envelope might become:

```rust
pub struct FactoryEvent {
    pub sequence: u64,
    pub sim_time: SimTime,
    pub event: Event,
}
```

Before implementing, discuss:

## Architectural decision: what moment does `sim_time` represent?

Possible meanings:

### A. State time after the update

If an update advances from 100 ms to 150 ms, every event produced by that update receives:

```text
sim_time = 150 ms
```

### B. Exact event occurrence time within the step

Example:

```text
update: 100 ms → 200 ms

sensor crossing occurred at 143 ms
```

This is more precise but requires event scheduling/interpolation logic.

### C. State time before the update

Usually less intuitive, but discuss it explicitly.

For Chunk 2, there are not yet physical events occurring within a time interval, so the simplest useful definition may be:

> Events created while the factory is at simulation time `T` receive timestamp `T`.

Then decide what happens for `create_part()`:

- Does part creation happen at the current factory time?
- Does calling `create_part()` advance time? Probably not.
- If two parts are created without an update between them, may they share a timestamp? Should sequence numbers distinguish them?

Record the semantics in comments or documentation.

---

# Step 5 — Introduce simulation clock state

Extend `Factory` with explicit simulation time.

Conceptually:

```rust
struct Factory {
    sim_time: SimTime,
    next_part_id: u64,
    next_event_sequence: u64,
}
```

Before implementing, discuss:

## Architectural decision: clock as field or separate object?

### Option A — direct `sim_time` field

Advantages:

- minimal,
- factory clearly owns simulation progress.

### Option B — dedicated `SimulationClock`

Example:

```rust
struct SimulationClock {
    now: SimTime,
}
```

Advantages:

- clock behavior could later be centralized.

Questions:

- Does the clock currently have enough behavior/invariants to deserve a type?
- Would extracting it improve the API today?
- Are we creating abstraction simply because "clock" sounds important?

Prefer the simplest representation that keeps invariants clear.

The simulation time field should remain private unless a public query is justified.

---

# Step 6 — Define the update API

This is one of the key decisions in Chunk 2.

We want the caller to explicitly advance time.

Possible APIs:

### Option A

```rust
fn update(&mut self, dt: Duration)
```

### Option B

```rust
fn update(&mut self, dt: Duration) -> Vec<FactoryEvent>
```

### Option C

```rust
fn advance_to(&mut self, time: SimTime)
```

### Option D

```rust
fn step(&mut self)
```

with a fixed internal timestep.

Discuss:

## Architectural decision: delta time vs absolute time?

Questions:

- Who should decide how much time advances?
- Does `update(dt)` make replay straightforward?
- Could `advance_to(time)` accidentally permit backward time?
- Will we eventually want both?
- Does a fixed `step()` hide an important decision from the caller?

For this project, favor explicit caller-supplied elapsed time unless a better argument emerges.

---

# Step 7 — Decide how updates return events

This is the first major state/event architecture decision promised at the end of Chunk 1.

Possible models:

## Option A — mutate and return events

```rust
fn update(&mut self, dt: Duration) -> Vec<FactoryEvent>
```

The simulator:

1. advances state,
2. records anything that happened,
3. returns resulting events.

Advantages:

- straightforward,
- caller immediately receives effects,
- deterministic and easy to test.

Questions:

- Does event allocation belong inside the simulator?
- Is returning a `Vec` acceptable if no events occur?
- Will this scale when many events occur?

## Option B — simulator stores an internal event queue

```rust
factory.update(dt);
let events = factory.drain_events();
```

Advantages:

- event production can occur from multiple methods.

Questions:

- Can callers forget to drain the queue?
- Does event lifetime/state become more complicated?
- What happens if the queue accumulates?

## Option C — callback/event sink

```rust
fn update(&mut self, dt: Duration, sink: &mut impl EventSink)
```

Advantages:

- avoids intermediate allocation,
- resembles later streaming.

Questions:

- Is this abstraction useful now?
- Does it complicate testing?
- Are we prematurely designing around future networking?

## Option D — pure transition result

Conceptually:

```rust
struct UpdateResult {
    events: Vec<FactoryEvent>,
}
```

Potentially extensible later, but may be unnecessary now.

Before implementation, ask me to choose.

The agent should recommend the simplest model that preserves a clear boundary and can be revisited later.

---

# Step 8 — Decide how non-update actions emit events

`create_part()` already emits `PartCreated`.

Once `update()` also emits events, we should make event production consistent.

Discuss:

## Architectural decision: should all mutating operations return events in the same shape?

Possibilities:

### A. `create_part() -> FactoryEvent`
and
`update() -> Vec<FactoryEvent>`

This is simple but asymmetric.

### B. Every state-changing operation returns `Vec<FactoryEvent>`

Example:

```rust
fn create_part(&mut self) -> Vec<FactoryEvent>
fn update(&mut self, dt: Duration) -> Vec<FactoryEvent>
```

Consistent, but a vector for a guaranteed single event may be noisy.

### C. Keep direct operations separate from time-driven update events

This may be perfectly reasonable.

Questions:

- Is consistency valuable enough to change a working API?
- Does `create_part()` conceptually represent a command/action while `update()` represents elapsed-time processing?
- Should part creation eventually become a command instead of a direct method?
- Is changing `create_part()` now useful, or premature?

Do not refactor purely for symmetry.

---

# Step 9 — Decide what zero-duration updates mean

Explicitly test the edge case:

```rust
factory.update(Duration::ZERO)
```

Possible semantics:

### A. valid no-op

- time does not change,
- no events occur unless some immediate work exists.

### B. invalid input

Reject or panic.

Discuss:

- Is a zero-duration update inherently nonsensical?
- Could it be useful for flushing immediate transitions later?
- Would accepting it simplify callers?
- What invariant would be violated?

Prefer documented behavior rather than accidental behavior.

---

# Step 10 — Decide how invalid time progression is handled

Depending on representation/API, consider:

- overflow,
- backwards absolute time,
- excessively large durations,
- arithmetic failure.

Do not over-engineer unlikely conditions, but make the invariant explicit:

> Simulation time must never move backwards.

If using `Duration` and checked arithmetic is available, discuss whether overflow should:

- panic,
- return `Result`,
- saturate,
- be considered unreachable for this project.

Do not introduce broad error infrastructure unless there is a real failure mode worth exposing.

---

# Step 11 — Add a way to observe current simulation time

Tests and future consumers may need to inspect current time.

Possible API:

```rust
fn sim_time(&self) -> SimTime
```

Discuss:

## Architectural decision: query method vs public field

Questions:

- Is current simulation time part of the simulator's public behavior?
- Does exposing it violate encapsulation?
- Would tests otherwise need to infer time indirectly from events?
- Does a getter preserve freedom to change internal representation later?

If a getter is added, keep the underlying field private.

---

# Step 12 — Make `create_part()` use current simulation time

Update the existing event creation path so a `PartCreated` event includes the current simulation time.

Test scenarios such as:

```text
Factory starts at 0 ms

create part A
→ event time = 0 ms

advance by 250 ms

create part B
→ event time = 250 ms
```

This establishes an important semantic rule:

> Actions that occur between updates occur at the factory's current simulation time.

Verify that event sequence numbers continue increasing independently of time.

For example:

```text
seq 0 @ 0 ms
seq 1 @ 0 ms
seq 2 @ 250 ms
```

Multiple events may share a timestamp.

That should be considered valid unless we decide otherwise.

---

# Step 13 — Implement the first deterministic `update`

At this point, there may still be no moving equipment.

That is okay.

The first `update` only needs to:

1. validate the requested time advance if necessary,
2. advance simulation time,
3. return the appropriate event collection.

If no time-driven behavior exists yet:

```text
update(100 ms)
→ time advances by 100 ms
→ zero events
```

This is intentionally small.

Do not invent fake heartbeat/tick events merely to make `update()` produce something.

Chunk 3 will introduce state whose behavior actually changes over elapsed time.

---

# Step 14 — Decide whether ticks themselves are events

Before adding any `Tick` or `TimeAdvanced` event, discuss this explicitly.

Possible reasons to emit one:

- external components might want every simulation step,
- diagnostics may want timing information.

Reasons not to:

- timestep is simulator execution detail,
- external behavior should be event-driven around meaningful domain changes,
- a 10 ms vs 20 ms internal timestep should not necessarily alter the public event stream.

Ask:

> If two simulations reach the same physical state using different step sizes, should consumers see different domain events merely because one used more updates?

For now, strongly consider **not** emitting tick events.

Record the decision.

---

# Step 15 — Add deterministic behavior tests

Chunk 2 should include tests around both time and reproducibility.

At minimum:

## Test A — initial time

A new/default factory starts at simulation time zero.

## Test B — time advances by delta

Example:

```text
initial = 0 ms
update(100 ms)
current = 100 ms
update(250 ms)
current = 350 ms
```

## Test C — zero-duration semantics

Test whatever behavior was explicitly chosen.

## Test D — created events use current simulation time

Example:

```text
create at 0 ms
advance 100 ms
create at 100 ms
```

## Test E — sequence and time are independent

Verify multiple events can share a timestamp while retaining unique monotonic sequences.

Example:

```text
seq 0 @ 0 ms
seq 1 @ 0 ms
seq 2 @ 100 ms
```

## Test F — deterministic replay

Create two fresh factories.

Feed them the same operations:

```text
create part
update 100 ms
create part
update 50 ms
update 25 ms
create part
```

Verify that observable results match.

Do not compare private fields directly.

Compare public outputs such as:

- returned events,
- current simulation time,
- IDs,
- sequence numbers.

If equality traits are needed on protocol types for these tests, discuss whether deriving them is justified now.

---

# Step 16 — Optional test: partitioned time advancement

This test is useful as a thinking exercise, but its expected result must be chosen carefully.

Compare:

```text
Factory A:
update(100 ms)

Factory B:
update(50 ms)
update(50 ms)
```

At the end, both should have the same current simulation time.

However, do **not** yet assume they must always produce identical domain events once motion is introduced.

Discuss the future issue:

- Some simulations are step-size independent.
- Some discrete update schemes are not.
- Exact crossing/event timing may require interpolation.

For Chunk 2, with no time-driven state, the final observable state should match.

Record this as a future design concern rather than solving it now.

---

# Step 17 — Review event ordering semantics

We now have two ordering concepts:

```text
sequence
sim_time
```

Discuss their roles.

A useful distinction may be:

- `sim_time` = when an event occurred in simulated time,
- `sequence` = total deterministic ordering when multiple events share a time.

Questions:

- Can two events share the same simulation time? Likely yes.
- Can two events share a sequence number? No.
- Can a later sequence have an earlier simulation time? Ideally no if time is monotonic.
- Should consumers sort by timestamp or trust stream order?
- Is sequence local to one factory simulation instance?

Do not add simulation-instance IDs yet unless there is a current need.

Document the chosen semantics.

---

# Step 18 — Review module organization

Now that simulator behavior is growing, inspect whether `main.rs` is becoming crowded.

Possible structures might eventually become:

```text
simulator/src/
├── main.rs
├── factory.rs
└── time.rs
```

But do not split files simply to match an imagined architecture.

Discuss:

- Is the current file difficult to navigate?
- Does a module represent a real conceptual boundary?
- Is `Factory` large enough to justify extraction?
- Would a `time` module contain enough behavior to be meaningful?

Refactor only if current complexity warrants it.

---

# Step 19 — Public API review

Before declaring Chunk 2 complete, inspect both crates.

Ask:

## Protocol

- Does `sim_time` have clearly documented units/semantics?
- Is the time type data-only?
- Did we accidentally add simulator behavior to `protocol`?
- Are events still simple schema objects?
- Are derives justified by actual use?

## Simulator

- Is `sim_time` private?
- Is current time queryable only through intentional public behavior?
- Is time advanced only through the defined update path?
- Can callers accidentally move time backwards?
- Does event production remain centralized enough to preserve sequence ordering?
- Did we expose internals only to make tests easier?

## Architecture

- Does deterministic replay work?
- Did wall-clock time sneak into the simulator?
- Did we introduce async/runtime concerns prematurely?
- Is every new abstraction paying for itself today?

---

# Chunk 2 definition of done

At the end of Chunk 2, the system should conceptually support:

```text
Factory
├── current simulation time
├── create_part()
└── update(dt)
```

with events conceptually containing:

```text
FactoryEvent
├── sequence
├── sim_time
└── event
```

Exact names and representations depend on architectural decisions made during implementation.

The simulator should satisfy this example behavior:

```text
factory created
time = 0

create part
→ PartCreated
→ sequence = 0
→ sim_time = 0

create part
→ PartCreated
→ sequence = 1
→ sim_time = 0

update by 100 ms
time = 100 ms

create part
→ PartCreated
→ sequence = 2
→ sim_time = 100 ms
```

And:

```bash
cargo test --workspace
```

must pass.

---

# Constraints for Chunk 2

Do **not** add:

- Tokio,
- async functions,
- NATS,
- WebSockets,
- serde unless an immediate testable need somehow emerges,
- wall-clock scheduling,
- threads,
- timers,
- conveyor movement,
- sensor logic,
- controller logic,
- persistence,
- a UI.

Do not call `sleep`.

Do not use `Instant::now()` or `SystemTime::now()` to drive simulation behavior.

The simulation clock must be controlled by inputs to the simulator.

---

# Guidance for the Codex agent

Continue using the collaboration style established in Chunk 1.

At every meaningful design choice:

1. identify the decision,
2. explain why it matters,
3. give 2–3 realistic options,
4. explain their tradeoffs,
5. make a recommendation when useful,
6. ask me to choose,
7. implement only after the choice is made.

Do not turn every implementation detail into an architectural debate.

Use judgment.

Good architectural decisions to surface in this chunk include:

- representation of simulation time,
- ownership of the time type,
- event timestamp semantics,
- delta-time vs absolute-time update API,
- how updates return events,
- zero-duration semantics,
- overflow/backwards-time handling,
- whether tick events should exist.

Implementation details that usually do not require stopping include:

- straightforward module imports,
- ordinary test naming,
- simple formatting,
- obvious compiler fixes that do not change architecture.

When reviewing code:

1. correctness,
2. deterministic behavior,
3. API boundaries,
4. state ownership,
5. event semantics,
6. simplicity,
7. style.

If the existing design is already reasonable, preserve it.

Do not refactor for abstraction's sake.

---

# Questions the completed design should be able to answer

Before moving on, I should be able to explain clearly:

1. What exactly does simulation time represent?
2. What Rust type represents it, and why?
3. Who owns the simulation clock?
4. Who is allowed to advance time?
5. What does an event's simulation timestamp mean?
6. Can multiple events have the same timestamp?
7. What does the event sequence number add beyond the timestamp?
8. What happens on `update(0)`?
9. Why is the simulation deterministic?
10. Why are we not using real-time clocks yet?
11. How does an update expose emitted events?
12. Which decisions might need revisiting when physical motion arrives?

If I cannot answer those confidently, Chunk 2 is not finished.

---

# What comes next

After Chunk 2 is complete and reviewed:

## Chunk 3 — Parts moving through conveyor segments

Chunk 3 should introduce the first state whose value changes as simulation time advances.

Likely topics include:

- representation of position,
- conveyor geometry or one-dimensional path coordinates,
- part ownership/location,
- movement based on elapsed time,
- boundary crossings,
- and the first real question about events that occur **inside** a timestep rather than exactly at its end.

That is where timestep size and event timing will begin to matter physically.

Do not begin Chunk 3 until Chunk 2 passes deterministic tests and the public API review.
