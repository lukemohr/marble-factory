# Chunk 3 — Parts Moving Through Conveyor Segments

This document continues the Marble Factory project after completion of Chunk 2.

Chunk 2 established:

- simulation time as elapsed `std::time::Duration`,
- caller-controlled, forward-only `Factory::update(dt)`,
- `FactoryEvent.sim_time` as event occurrence time,
- factory-local sequence numbers for total event ordering,
- `update(dt) -> Vec<FactoryEvent>`,
- direct `create_part() -> FactoryEvent`,
- valid zero-duration updates,
- checked clock advancement,
- deterministic replay tests,
- no tick events,
- and an internal `factory` module inside the binary `simulator` crate.

Chunk 3 introduces the first state whose value changes as simulation time advances.

The focus is deliberately narrow:

> Put parts onto a single conveyor segment and move them deterministically as time advances.

Do **not** add sensors, diverters, routing logic, async execution, NATS, or UI yet.

The purpose of this chunk is to practice modeling:

- physical state,
- ownership/location,
- continuous motion using discrete updates,
- boundary crossing,
- event occurrence inside a timestep,
- and timestep-partition invariance.

---

# Chunk 3 goal

At the end of Chunk 3, the simulator should be able to:

1. represent at least one conveyor segment,
2. place a part onto that conveyor,
3. represent a part's position along the conveyor,
4. move parts according to conveyor speed and elapsed simulation time,
5. detect when a part reaches the end of the conveyor,
6. emit meaningful domain events for conveyor entry and exit,
7. assign physically meaningful simulation timestamps to boundary-crossing events,
8. produce the same physical outcome whether time is advanced in one large step or several smaller steps, for the modeled behavior.

A target interaction might look conceptually like:

```text
factory time = 0 s

create part
→ PartCreated(part=0) @ 0 s

place part 0 onto conveyor 0
→ PartEnteredConveyor(part=0, conveyor=0) @ 0 s

conveyor:
    length = 10 m
    speed  = 2 m/s

update(3 s)

part 0 position = 6 m
factory time = 3 s

update(3 s)

part reaches conveyor end at t = 5 s
→ PartExitedConveyor(part=0, conveyor=0) @ 5 s

factory time = 6 s
```

Notice the important detail:

> The update ends at 6 s, but the physical event occurred at 5 s.

That is the central architecture problem in this chunk.

---

# Progress record

Add decisions here as the chunk proceeds.

| Step | Status | Decision / outcome |
| --- | --- | --- |
| 0 — Review Chunk 2 baseline | Complete | `FactoryEvent` has sequence, elapsed `Duration` time, and payload; the internal `Factory` owns a private clock and returns an empty `Vec` from deterministic updates. `create_part()` is timestamped, the factory lives in `simulator::factory`, tests pass, and no dependencies were added. |
| 1 — Define the first physical model | Complete | Chose one straight conveyor with a 1D, conveyor-local distance coordinate. Future 2D layout maps conveyor geometry and local distance to world coordinates; it does not replace transport position. |
| 2 — Decide conveyor identity/config representation | Complete | `Factory` will hold one internal `Conveyor` with only behavioral data (`DeviceId`, length, speed), not a protocol descriptor. This is deliberately temporary topology; device IDs keep later multi-device/routing expansion localized. |
| 3 — Decide position representation | Complete | Chose internal `f64` meters for length and position. Configuration values will be finite and non-negative; tests compare derived positions using a tolerance. |
| 4 — Decide speed representation | Complete | Speed is a private mutable `f64` meters/second operating state, initialized finite and positive for Chunk 3. Length remains fixed configuration. Future controls may permit zero speed; reverse motion is deferred. |
| 5 — Decide part location/state ownership | Complete | `Factory` will own one explicit `Part { id, location }` record per part. `PartLocation` represents lifecycle state, prevents duplicate placement, and offers a clean future persistence mapping without adding persistence now. |
| 6 — Create versus place | Complete | `create_part()` creates `Unplaced` simulator state and emits `PartCreated`; a separate placement operation will perform the physical transition and emit entry. |
| 7 — Conveyor placement API | Complete | Chose `place_part_on_conveyor(part_id, conveyor_id) -> Result<FactoryEvent, PlacementError>`. The conveyor-specific destination is truthful today and leaves future transfers to real destination semantics rather than a generic device API. |
| 8 — Conveyor protocol events | Complete | Add `PartEnteredConveyor { part_id, conveyor_id }` and `PartExitedConveyor { part_id, conveyor_id }`. Positions are implicit in lifecycle/configuration, and envelope `sim_time` is the occurrence time. |
| 9 — Routine motion events | Complete | Do not add `PartMoved`. Routine numerical position updates are state, keeping domain event history independent of timestep; future visualization uses snapshots or separate telemetry. |
| 10 — Boundary semantics | Complete | Treat parts as points: exit occurs exactly at `position == length`; valid `OnConveyor` position is `0 ≤ position < length`, and placement starts at zero. |
| 11 — In-step crossing time | Complete | Interpolate exact exit time: update start plus `(length − position) / speed`. Events are not stamped at the update end, preserving partition-invariant physical time. |
| 12 — Crossing-time conversion | Complete | Convert validated finite non-negative `f64` travel time with `Duration::from_secs_f64`, keeping one simulation-time representation. |
| 13 — Post-exit state | Complete | Use terminal `PartLocation::Exited` and commit the transition before returning `PartExitedConveyor`; provenance is event history, while transfer states wait for real routing. |
| 14 — Part-state query | Complete | Add internal `part_location(part_id) -> Option<PartLocation>` as the intentional read model. It exposes lifecycle state without leaking collections or creating a protocol snapshot prematurely. |
| 15 — Multiple-exit ordering | Complete | Detect all exits, sort by crossing time then numeric `PartId`, commit `Exited`, and allocate sequences in that final order. `PartId::value()` was added for this concrete tie-breaker without deriving `Ord`. |
| 16 — Motion implementation and tests | Complete | Added internal `Conveyor`, `Part`, and `PartLocation` modules; lifecycle events, validated placement, speed × time motion, precise in-step exits, partition invariance, non-round conversion, and multiple-part ordering are covered by tests. |
| 17 — Update API review | Complete | Keep `update(dt) -> Vec<FactoryEvent>`. Multiple exact-time exits are detected, sorted, committed, and returned naturally; no queue or callback need emerged. |
| 18 — Protocol and public API review | Pending final review | |
| 7 — Add conveyor-related protocol events | Not started | |
| 8 — Implement motion within `update(dt)` | Not started | |
| 9 — Decide boundary-crossing semantics | Not started | |
| 10 — Compute exact crossing time inside an update | Not started | |
| 11 — Decide post-exit part state | Not started | |
| 12 — Test timestep partitioning | Not started | |
| 13 — Review event ordering for same-time crossings | Not started | |
| 14 — Public API and module review | Not started | |

---

# Step 0 — Review the current baseline

Before changing code, inspect the current implementation.

Confirm:

- `protocol::FactoryEvent` contains `sequence`, `sim_time: Duration`, and `event`,
- `Factory` owns private `sim_time`,
- `Factory::update(dt)` advances the clock deterministically,
- `update()` currently returns an empty `Vec`,
- `create_part()` emits a timestamped `PartCreated`,
- `Factory` lives in the internal `factory` module,
- all workspace tests pass,
- no external dependencies have been added.

Do not refactor working Chunk 2 code without a concrete Chunk 3 need.

Summarize the current simulator state before implementation.

---

# Step 1 — Define the first physical model

Keep the factory topology intentionally minimal.

For Chunk 3, begin with **one straight conveyor segment**.

Conceptually:

```text
entry
  │
  ▼
0 m ───────────────────────────── length
            conveyor
```

A part has only one relevant spatial coordinate:

```text
distance from conveyor start
```

No 2D geometry is required yet.

Before implementing, discuss:

## Architectural decision: 1D path coordinate vs 2D world position

### Option A — 1D conveyor-local coordinate

Example:

```rust
position = 2.4 // meters from start
```

Advantages:

- directly models what matters,
- easy boundary calculations,
- avoids geometry complexity,
- naturally supports later conveyor graphs.

### Option B — global 2D coordinates

Example:

```rust
Vec2 { x, y }
```

Advantages:

- closer to eventual visualization,
- useful for arbitrary geometry.

Disadvantages now:

- introduces geometry concerns unrelated to transport architecture,
- conveyor movement still needs path-relative state.

Strongly consider conveyor-local 1D position for Chunk 3.

Record the choice.

---

# Step 2 — Decide how conveyors are represented

The protocol already has `DeviceId` and `DeviceKind::Conveyor`.

Now the simulator needs actual conveyor behavior/configuration.

A possible internal type:

```rust
struct Conveyor {
    id: DeviceId,
    length: ...,
    speed: ...,
}
```

Before implementation, discuss:

## Architectural decision: descriptor data vs simulator object

Questions:

- Should `DeviceDescriptor` be embedded in the simulator's `Conveyor`?
- Or should the simulator hold only the fields it actually needs?
- Does the protocol descriptor represent identity metadata while `Conveyor` represents behavior?
- Should `Conveyor` be visible outside the simulator module?

Avoid turning protocol types into behavioral objects.

Also decide whether Chunk 3 needs:

### Option A — exactly one conveyor field on `Factory`

Example:

```rust
conveyor: Conveyor
```

### Option B — a collection keyed by `DeviceId`

Example:

```rust
HashMap<DeviceId, Conveyor>
```

Questions:

- Are multiple conveyors needed for the current behavior?
- Would a single conveyor make Chunk 4/5 unnecessarily hard to extend?
- Does introducing a map now provide real architectural value?
- Would a `Vec` be enough if IDs are not yet used for lookup?

Do not automatically choose the most general collection.

---

# Step 3 — Decide length and position representation

We need physical distance.

Possible choices:

## Option A — `f64` meters

Example:

```rust
length_m: f64
position_m: f64
```

Advantages:

- natural for physical calculations,
- simple speed × time arithmetic,
- flexible precision.

Concerns:

- floating-point comparisons,
- exact equality in tests,
- NaN and negative values.

## Option B — integer millimeters

Example:

```rust
length_mm: u64
position_mm: u64
```

Advantages:

- exact arithmetic,
- easy comparisons.

Concerns:

- time × speed may still create fractional movement,
- unit is fixed into the model,
- sub-millimeter motion may become awkward.

## Option C — domain newtypes around floating point

Example:

```rust
struct Distance(f64);
```

Advantages:

- unit semantics are explicit,
- reduces accidental mixing.

Questions:

- Is the additional wrapper useful today?
- Do we already have invariants that justify controlled construction?

Before choosing, discuss how precision and testability should be handled.

If using `f64`, establish a testing policy such as tolerance-based comparison instead of direct equality for derived positions.

Do not add a units crate in this chunk.

---

# Step 4 — Decide speed representation

The conveyor needs a deterministic speed.

A simple model:

```text
constant positive speed
```

No acceleration, ramp-up, motor dynamics, or start/stop behavior yet.

Possible representation:

```rust
speed_m_per_s: f64
```

Discuss:

## Architectural decision: configuration invariant enforcement

Questions:

- Can conveyor length be zero?
- Can speed be zero?
- Can speed be negative?
- Should invalid configuration be prevented at construction?
- Does that justify a constructor returning `Result`?
- Or are these fixed test/demo values internal enough that assertions are sufficient for now?

Use the smallest validation approach that makes invalid state difficult to create.

Do not create a generic validation/error framework.

---

# Step 5 — Decide how part physical state is represented

Chunk 1 only needed an ID counter.

Now created parts need simulator-owned state.

Conceptually, a part may be:

```text
Created but not on equipment

or

On conveyor 0 at position 3.2 m

or

Exited conveyor 0
```

Before implementation, discuss:

## Architectural decision: separate part state object or parallel collections?

### Option A — explicit simulator `Part`

Example:

```rust
struct Part {
    id: PartId,
    location: PartLocation,
}
```

with:

```rust
enum PartLocation {
    Unplaced,
    OnConveyor {
        conveyor_id: DeviceId,
        position_m: f64,
    },
    Exited,
}
```

Advantages:

- location is explicit,
- invalid combinations are harder to represent,
- future locations can become variants.

### Option B — conveyor owns the parts currently on it

Example:

```rust
Conveyor {
    parts: Vec<PartOnConveyor>,
}
```

Advantages:

- localizes moving state to equipment.

Questions:

- Where does an unplaced part live?
- Who owns part identity globally?
- What happens when a part transfers between conveyors later?

### Option C — factory-level maps for part state and placement

Potentially flexible but can allow inconsistent parallel data.

Discuss which object should be the authoritative owner of a part's current location.

This is an important architecture decision because future transfer/routing logic will build on it.

---

# Step 6 — Update `create_part()` to create simulator state

Currently `create_part()` allocates an ID and emits an event.

Once the factory tracks parts, it should also create the corresponding internal state.

Before editing, discuss:

## Architectural decision: what does "created" mean physically?

Possible semantics:

### A. Created means an unplaced part exists

```text
PartCreated
location = Unplaced
```

Then a separate operation places it onto equipment.

### B. Creating a part automatically places it on the entry conveyor

Simpler for simulation demos, but combines two domain actions.

For architecture practice, strongly consider keeping:

```text
create
```

and:

```text
place/enter conveyor
```

as separate actions.

That gives distinct lifecycle events and clearer responsibilities.

Record the choice.

---

# Step 7 — Decide how a part enters a conveyor

We need an operation that moves an existing part from an unplaced state onto the conveyor.

Possible API:

```rust
fn place_part_on_conveyor(
    &mut self,
    part_id: PartId,
    conveyor_id: DeviceId,
) -> ...
```

or a narrower current API:

```rust
fn place_part_on_entry_conveyor(&mut self, part_id: PartId) -> ...
```

Discuss:

## Architectural decision: general API vs current topology-specific API

Questions:

- Does a general conveyor ID parameter reflect an already-real concept?
- If there is only one conveyor, is it needless generality?
- Will multiple conveyors arrive soon enough that a general API avoids immediate churn?
- Should placement fail if the part is already placed or unknown?

Also discuss return semantics:

- `FactoryEvent` if successful and failure is considered impossible,
- `Result<FactoryEvent, Error>` if invalid caller actions are meaningful,
- panic/assertion if misuse is strictly programmer error.

This is likely the first operation where invalid state transitions are realistic:

```text
unknown part
already on conveyor
already exited
unknown conveyor
```

Do not hide this decision.

---

# Step 8 — Add conveyor lifecycle events to `protocol`

The event stream should now represent meaningful physical transitions.

Likely events:

```rust
Event::PartEnteredConveyor {
    part_id: PartId,
    conveyor_id: DeviceId,
}
```

and:

```rust
Event::PartExitedConveyor {
    part_id: PartId,
    conveyor_id: DeviceId,
}
```

Before implementing, discuss:

## Architectural decision: does position belong in these events?

For example:

```rust
PartEnteredConveyor {
    part_id,
    conveyor_id,
    position,
}
```

Questions:

- Is entry position always implicitly zero?
- Is exit position always implicitly conveyor length?
- Would including redundant physical values improve diagnostics?
- Or does it duplicate configuration/state unnecessarily?

Prefer events that communicate the domain transition without unnecessary repeated data.

Keep these protocol types data-only.

---

# Step 9 — Decide whether routine motion itself emits events

A part moving from 1.0 m to 1.2 m is state change.

Should it emit a domain event?

Possible options:

### A. emit every position update

Example:

```text
PartMoved(part=0, position=1.2)
```

Advantages:

- easy external visualization.

Disadvantages:

- event volume depends on update frequency,
- public domain stream becomes tied to timestep,
- changing `dt` changes externally visible behavior.

### B. emit only meaningful transitions

For Chunk 3:

- entered conveyor,
- exited conveyor.

Position remains queryable simulator state.

This better preserves the Chunk 2 decision not to expose ticks.

Discuss:

> Should domain events describe every numerical state update, or meaningful changes in the factory lifecycle?

Strongly consider **not** emitting `PartMoved` events yet.

Future HMI state streaming can be solved separately from domain-event history.

Record the decision.

---

# Step 10 — Implement deterministic motion

For a part currently on a conveyor:

```text
distance_traveled = speed × elapsed_time
```

If using meters and seconds:

```rust
delta_position = speed_m_per_s * dt.as_secs_f64();
```

At first glance:

```rust
new_position = old_position + delta_position;
```

But do not simply clamp and move on.

The update must distinguish:

```text
part remains on conveyor
```

from:

```text
part reaches conveyor end during this update
```

The factory clock should still advance by the complete caller-supplied `dt`.

Example:

```text
start time = 4.0 s
update dt  = 2.0 s
end time   = 6.0 s

part:
    old position = 8 m
    conveyor speed = 2 m/s
    length = 10 m

crossing occurs after 1 s

event time = 5.0 s
```

That event time should not automatically become the end-of-update time.

---

# Step 11 — Decide exact boundary semantics

We need a precise definition for "exited."

Discuss:

## Architectural decision: when is a part considered to have left the conveyor?

Possible definitions:

### A. `position >= length`

The exact instant the leading point reaches the endpoint.

### B. `position > length`

Requires motion beyond the boundary.

### C. Part geometry-aware exit

Would require radius/length of the physical part.

For Chunk 3, we do not yet model part dimensions.

Use a point-particle interpretation unless we explicitly decide otherwise.

Also decide:

- If a part begins exactly at `position == length`, should it already be exited?
- Should valid `OnConveyor` state enforce `0 <= position < length`?
- Can a part be manually placed anywhere except zero?

Make the invariant explicit.

---

# Step 12 — Compute event occurrence time inside the update

This is the key technical step.

If:

```text
remaining_distance = conveyor_length - old_position
speed > 0
```

then:

```text
time_to_exit = remaining_distance / speed
```

If:

```text
time_to_exit <= dt
```

the part exits during the update.

The event should receive:

```text
event_time = update_start_time + time_to_exit
```

rather than:

```text
update_end_time
```

Before implementation, discuss:

## Architectural decision: precise crossing time vs timestep-end approximation

### Option A — interpolate exact crossing time

Advantages:

- physically meaningful event timestamps,
- timestep partitioning can produce the same transition time,
- later sensors can use the same approach.

### Option B — stamp at end of update

Advantages:

- simpler.

Disadvantages:

- event time depends on chosen timestep,
- 1-second and 100-ms simulation steps produce different event timestamps.

Strongly prefer exact crossing time for this project.

---

# Step 13 — Decide how floating physical time becomes `Duration`

If position/speed use `f64`, then:

```text
time_to_exit
```

will probably be a floating number of seconds.

But event time is stored as:

```rust
Duration
```

Discuss:

## Architectural decision: conversion and precision policy

Questions:

- Use `Duration::from_secs_f64(...)`?
- What precision can be represented?
- What happens for non-finite values?
- Can invalid configuration make this panic?
- Is conversion deterministic enough for our intended tests?

Define the expected invariants so conversion receives valid finite non-negative values.

Avoid introducing a second simulation-time type purely because this calculation uses floats unless the design genuinely needs it.

---

# Step 14 — Decide update ordering: move state first or emit first?

For an exiting part, the simulator needs to:

1. calculate exit time,
2. transition internal state,
3. emit `PartExitedConveyor`.

Discuss:

## Architectural decision: what state should hold when an event is observed?

Even though events are returned only after `update()` completes, define the conceptual rule:

> An event describes a transition that the simulator has committed.

This usually implies the state transition and event creation should be handled together.

Avoid architectures where an event claims a part exited but the simulator still considers it on the conveyor after the update.

---

# Step 15 — Decide post-exit part state

After a part reaches the end, where does it go?

Chunk 3 should not yet transfer it to another conveyor or bin.

Possible states:

### A. `Exited`

Generic terminal state.

### B. `OffConveyor { from: DeviceId }`

Retains provenance.

### C. `AwaitingTransfer { from: DeviceId }`

Anticipates Chunk 4/5 routing.

Questions:

- What is true today?
- Is "awaiting transfer" already a real modeled behavior, or future speculation?
- Does retaining the previous conveyor ID help?
- Is event history sufficient for provenance?

Choose the smallest state that accurately describes the current system.

Do not invent a routing queue yet.

---

# Step 16 — Decide how to query part state

Tests need a public-behavior way to inspect where a part is.

Possible API:

```rust
fn part_location(&self, part_id: PartId) -> Option<...>
```

or:

```rust
fn part_position(&self, part_id: PartId) -> Option<f64>
```

Discuss:

## Architectural decision: expose simulator domain type or narrow query methods?

Questions:

- Should callers see an internal `PartLocation` enum?
- Would making it public prematurely expose simulator implementation?
- Would specific read-only queries be awkward as more states arrive?
- Is another protocol snapshot type warranted? Probably not yet.

The tests should not gain direct access to internal collections.

Choose an intentional read model.

---

# Step 17 — Handle multiple parts

Even with one conveyor, test more than one part.

Initially, ignore collisions and spacing.

Parts may mathematically overlap or pass through the same coordinate.

That is intentional.

Discuss briefly:

> Is collision/spacing part of the current model?

For Chunk 3: likely no.

The conveyor transports independent point-like parts at identical speed.

This lets architecture and event timing remain the focus.

Do not introduce queueing, blocking, minimum spacing, or collision physics.

---

# Step 18 — Decide event ordering when multiple parts exit

Consider:

```text
two parts exit during the same update
```

They may have:

- different crossing times,
- the same crossing time.

The event vector should have deterministic ordering.

Discuss:

## Architectural decision: ordering events generated inside one update

A reasonable rule:

1. order by event `sim_time`,
2. break equal-time ties deterministically,
3. assign sequence numbers in that final order.

Possible tie-breakers:

- `PartId`,
- current storage order,
- conveyor ID then part ID.

Avoid relying accidentally on nondeterministic map iteration.

This matters if parts are stored in a `HashMap`.

Record the rule.

---

# Step 19 — Be careful with mutation while discovering events

Depending on the chosen storage structure, updating parts and producing chronological events may be awkward.

For example:

```rust
for part in parts.iter_mut() {
    ...
}
```

may produce events in storage iteration order rather than event-time order.

Discuss implementation approaches only after the data model is chosen.

Possible pattern:

1. compute transition candidates,
2. sort candidates by occurrence time/tie-breaker,
3. commit/update state,
4. allocate event sequence numbers in deterministic order.

Do not build a generic scheduler yet.

But if exact event ordering requires separating detection from event allocation, recognize that explicitly.

---

# Step 20 — Test basic movement

Add tests for:

## Test A — newly placed part starts at conveyor entry

Example:

```text
position = 0
```

## Test B — part moves according to speed × time

Example:

```text
length = 10 m
speed = 2 m/s

update 1.5 s

position ≈ 3 m
```

Use tolerance-based comparisons if position is floating point.

## Test C — time and motion advance together

Verify:

```text
factory sim_time = expected
part position = expected
```

after repeated updates.

---

# Step 21 — Test no premature exit

Example:

```text
length = 10 m
speed = 2 m/s

update 4.9 s
```

Expected:

- part remains on conveyor,
- no `PartExitedConveyor`,
- position ≈ 9.8 m.

---

# Step 22 — Test exact boundary exit

Example:

```text
length = 10 m
speed = 2 m/s

update 5 s
```

Expected:

```text
PartExitedConveyor @ 5 s
```

and the part is no longer in the `OnConveyor` state.

---

# Step 23 — Test crossing inside a larger update

This is a key test.

Example:

```text
time = 0
position = 0
length = 10 m
speed = 2 m/s

update(8 s)
```

Expected:

- factory time becomes `8 s`,
- exit event occurs at `5 s`,
- part is exited after the update.

Do not stamp the event at `8 s`.

---

# Step 24 — Test timestep partition invariance

Compare two fresh factories.

## Run A

```text
place part
update(8 s)
```

## Run B

```text
place part
update(2 s)
update(2 s)
update(2 s)
update(2 s)
```

Expected physical domain outcome:

- same final factory time,
- same final part state,
- same physical exit occurrence time.

The domain event sequence should describe the same entry/exit lifecycle, even though the number of update calls differs.

This test is very important because Chunk 2 deliberately avoided making timestep itself part of the event stream.

---

# Step 25 — Test a non-round crossing time

Avoid testing only exact whole seconds.

Example:

```text
length = 1 m
speed = 3 m/s

time_to_exit = 1/3 s
```

If using floating conversion to `Duration`, decide how strict the expected timestamp comparison should be.

This test should reveal the practical precision semantics of the chosen representation.

---

# Step 26 — Test multiple part ordering

Create multiple parts with different starting conditions or entry times so they exit during the same update.

Verify:

- chronological event order,
- monotonic sequence numbers,
- deterministic equal-time tie-breaking if applicable.

Run the same scenario multiple times and verify identical output.

---

# Step 27 — Review whether `update()` is still conceptually sound

After implementing real time-driven behavior, revisit the Chunk 2 API:

```rust
fn update(&mut self, dt: Duration) -> Vec<FactoryEvent>
```

Ask:

- Does this still feel natural?
- Does returning a `Vec` adequately represent multiple transitions?
- Did exact crossing-time events force any awkwardness?
- Is there an actual need yet for an event queue/sink?
- Is the API still deterministic and easy to test?

Do not change it merely because alternatives exist.

Only refactor if Chunk 3 exposed a concrete weakness.

---

# Step 28 — Review protocol boundaries

The protocol now likely has:

```text
PartCreated
PartEnteredConveyor
PartExitedConveyor
```

Review:

- Are these genuine domain events?
- Are simulator-only values leaking into the protocol?
- Is position omitted or included for a justified reason?
- Does `Duration` still make sense as the event timestamp contract?
- Are events independent of update/tick frequency?

Avoid adding a `PartMoved` event solely for visualization.

---

# Step 29 — Review simulator module structure

Chunk 3 may justify additional internal modules.

Possible shape:

```text
simulator/src/
├── main.rs
├── factory.rs
├── conveyor.rs
└── part.rs
```

But create modules based on actual cohesion, not desired aesthetics.

Discuss:

- Does `Conveyor` have enough behavior to own its own module?
- Is `PartLocation` logically separate?
- Should update orchestration stay in `Factory`?
- Which type owns physical transition calculations?

A useful guiding principle:

> `Factory` coordinates the simulation; domain objects may own local rules where that simplifies invariants.

Do not distribute behavior across modules merely to make files shorter.

---

# Step 30 — Public API review

Before declaring Chunk 3 complete, inspect the architecture.

## State ownership

Be able to explain:

- Who owns parts?
- Who owns conveyors?
- Who owns part location?
- Who is allowed to mutate position?
- Can a part be on two conveyors at once?

## Time

Be able to explain:

- Why the factory clock advances to the end of `dt`,
- while an event can have a timestamp inside that interval.

## Events

Be able to explain:

- why entry/exit are domain events,
- why routine movement is not currently a domain event,
- how same-time events receive deterministic sequence order.

## Determinism

Verify no result depends on:

- wall-clock time,
- hash iteration order,
- thread scheduling,
- random numbers,
- update-call count when domain behavior is equivalent.

## Encapsulation

Check:

- internal maps/collections remain private,
- tests use intentional queries,
- protocol remains schema-only,
- simulator state types are not exposed without a reason.

---

# Chunk 3 definition of done

At completion, the project should conceptually support:

```text
Factory
├── simulation time
├── parts
├── conveyor(s)
├── create_part()
├── place_part_on_conveyor(...)
└── update(dt)
```

with a lifecycle such as:

```text
PartCreated
      │
      ▼
   Unplaced
      │
      │ place
      ▼
PartEnteredConveyor
      │
      ▼
 OnConveyor(position)
      │
      │ time advances
      ▼
PartExitedConveyor
      │
      ▼
    Exited
```

A representative scenario should work:

```text
t = 0
create part 0
→ PartCreated @ 0

place part 0 on conveyor 0
→ PartEnteredConveyor @ 0

conveyor:
    length = 10 m
    speed = 2 m/s

update 3 s
→ part position = 6 m
→ no event
→ factory time = 3 s

update 3 s
→ part exits at physical time 5 s
→ PartExitedConveyor @ 5 s
→ factory time = 6 s
```

And:

```bash
cargo test --workspace
```

must pass.

---

# Constraints for Chunk 3

Do **not** add:

- Tokio,
- async,
- NATS,
- WebSockets,
- UI,
- sensors,
- diverters,
- bins,
- routing,
- PLC/controller logic,
- conveyor acceleration,
- conveyor start/stop commands,
- collisions between parts,
- part dimensions,
- randomized behavior,
- persistence,
- wall-clock timing,
- a physics engine,
- a generic event scheduler unless exact crossing behavior demonstrably requires one.

No `sleep`.

No `Instant::now()` or `SystemTime::now()` for simulation behavior.

No update-frequency-driven `PartMoved` event stream unless we explicitly reverse the architectural decision after discussion.

---

# Guidance for the Codex agent

Continue the established collaboration style.

At meaningful architectural choices:

1. identify the decision,
2. explain why it matters,
3. present realistic alternatives,
4. explain tradeoffs,
5. recommend one when appropriate,
6. ask me to choose,
7. then implement or guide implementation.

Important decisions to surface in Chunk 3 include:

- 1D path coordinate vs 2D geometry,
- conveyor storage structure,
- distance representation,
- speed representation and invariants,
- ownership of part location,
- created vs placed semantics,
- placement failure handling,
- conveyor entry/exit event payloads,
- whether routine motion emits events,
- exact crossing timestamp semantics,
- floating-to-`Duration` precision,
- post-exit state,
- public part-state query design,
- deterministic ordering of multiple crossings.

Do not stop for every syntax-level decision.

When reviewing code, prioritize:

1. correctness,
2. physical/event-time semantics,
3. determinism,
4. state ownership,
5. invalid-state prevention,
6. protocol boundaries,
7. simplicity,
8. style.

If a simple implementation remains correct and clear, keep it.

---

# Questions the completed design should be able to answer

Before moving to Chunk 4, I should be able to explain:

1. What does a part's position mean?
2. Who owns that position?
3. What units are used for distance and speed?
4. Why were those representations chosen?
5. What makes a part "on a conveyor"?
6. Can a created part exist without being placed?
7. What prevents a part from being on multiple conveyors?
8. What exactly does `PartEnteredConveyor` mean?
9. What exactly does `PartExitedConveyor` mean?
10. Why does normal movement not necessarily emit an event?
11. If `update()` ends at 6 s but a part crosses the boundary at 5 s, why is the event stamped 5 s?
12. How is that crossing time calculated?
13. What happens to the part after exit?
14. How are multiple events inside one update ordered?
15. Why does `update(8 s)` produce the same physical transition time as four `update(2 s)` calls?
16. Which parts of this design will likely change once conveyors connect to one another?

If these answers are unclear, Chunk 3 is not finished.

---

# What comes next

After Chunk 3 is complete and reviewed:

## Chunk 4 — Sensors and observable factory events

Chunk 4 should introduce a sensor positioned along a conveyor.

That will build directly on the crossing-time work from Chunk 3.

A part may cross a sensor inside an update:

```text
update interval: 2.0 s → 3.0 s

sensor crossing: 2.37 s
conveyor exit:   2.91 s
```

The simulator will then need to emit multiple physical events from one update in correct chronological order.

Likely topics:

- sensor placement,
- sensor identity,
- crossing detection,
- event ordering within one update,
- multiple sensors,
- and the distinction between physical state and externally observed signals.

Do not begin Chunk 4 until Chunk 3 passes its deterministic partitioning tests and architecture review.
