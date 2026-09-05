use std::time::Duration;

use protocol::{DeviceId, Event, FactoryEvent, PartId};

use crate::conveyor::Conveyor;
use crate::part::{Part, PartLocation};

const ENTRY_CONVEYOR_ID: DeviceId = DeviceId::new(0);
const ENTRY_CONVEYOR_LENGTH_M: f64 = 10.0;
const ENTRY_CONVEYOR_SPEED_M_PER_S: f64 = 2.0;

/// The in-memory state of the simulated factory.
// The binary gains its runtime loop in a later chunk; tests exercise this state today.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) struct Factory {
    sim_time: Duration,
    next_part_id: u64,
    next_event_sequence: u64,
    conveyor: Conveyor,
    parts: Vec<Part>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl Factory {
    /// Creates an empty factory with one 10 m conveyor moving at 2 m/s.
    pub(super) fn new() -> Self {
        Self::with_conveyor(Conveyor::new(
            ENTRY_CONVEYOR_ID,
            ENTRY_CONVEYOR_LENGTH_M,
            ENTRY_CONVEYOR_SPEED_M_PER_S,
        ))
    }

    fn with_conveyor(conveyor: Conveyor) -> Self {
        Self {
            sim_time: Duration::ZERO,
            next_part_id: 0,
            next_event_sequence: 0,
            conveyor,
            parts: Vec::new(),
        }
    }

    /// Creates an unplaced part and returns the event describing that creation.
    pub(super) fn create_part(&mut self) -> FactoryEvent {
        let part_id = PartId::new(self.next_part_id);
        self.next_part_id += 1;
        self.parts.push(Part::new(part_id));

        self.emit(Event::PartCreated { part_id })
    }

    /// Places an unplaced part at the entry of the identified conveyor.
    pub(super) fn place_part_on_conveyor(
        &mut self,
        part_id: PartId,
        conveyor_id: DeviceId,
    ) -> Result<FactoryEvent, PlacementError> {
        if conveyor_id != self.conveyor.id() {
            return Err(PlacementError::UnknownConveyor(conveyor_id));
        }

        let part = self
            .parts
            .iter_mut()
            .find(|part| part.id == part_id)
            .ok_or(PlacementError::UnknownPart(part_id))?;

        if part.location != PartLocation::Unplaced {
            return Err(PlacementError::PartNotUnplaced(part_id));
        }

        part.location = PartLocation::OnConveyor {
            conveyor_id,
            position_m: 0.0,
        };

        Ok(self.emit(Event::PartEnteredConveyor {
            part_id,
            conveyor_id,
        }))
    }

    /// Returns the factory's current elapsed simulation time.
    pub(super) fn sim_time(&self) -> Duration {
        self.sim_time
    }

    /// Returns the current lifecycle and location state for a part.
    pub(super) fn part_location(&self, part_id: PartId) -> Option<PartLocation> {
        self.parts
            .iter()
            .find(|part| part.id == part_id)
            .map(|part| part.location)
    }

    /// Advances simulation time and returns any events caused by that interval.
    pub(super) fn update(&mut self, dt: Duration) -> Vec<FactoryEvent> {
        let update_start_time = self.sim_time;
        let update_end_time = self
            .sim_time
            .checked_add(dt)
            .expect("simulation time overflow");
        let dt_seconds = dt.as_secs_f64();
        let mut exits = Vec::new();

        for part in &mut self.parts {
            let PartLocation::OnConveyor {
                conveyor_id,
                position_m,
            } = part.location
            else {
                continue;
            };

            debug_assert_eq!(conveyor_id, self.conveyor.id());

            let time_to_exit_seconds =
                (self.conveyor.length_m() - position_m) / self.conveyor.speed_m_per_s();

            if time_to_exit_seconds <= dt_seconds {
                let exit_time = update_start_time
                    .checked_add(Duration::from_secs_f64(time_to_exit_seconds))
                    .expect("simulation time overflow");

                part.location = PartLocation::Exited;
                exits.push(ExitCandidate {
                    part_id: part.id,
                    conveyor_id,
                    sim_time: exit_time,
                });
            } else {
                part.location = PartLocation::OnConveyor {
                    conveyor_id,
                    position_m: position_m + self.conveyor.speed_m_per_s() * dt_seconds,
                };
            }
        }

        exits.sort_by(|left, right| {
            left.sim_time
                .cmp(&right.sim_time)
                .then_with(|| left.part_id.value().cmp(&right.part_id.value()))
        });

        self.sim_time = update_end_time;

        exits
            .into_iter()
            .map(|exit| {
                self.emit_at(
                    exit.sim_time,
                    Event::PartExitedConveyor {
                        part_id: exit.part_id,
                        conveyor_id: exit.conveyor_id,
                    },
                )
            })
            .collect()
    }

    fn emit(&mut self, event: Event) -> FactoryEvent {
        self.emit_at(self.sim_time, event)
    }

    fn emit_at(&mut self, sim_time: Duration, event: Event) -> FactoryEvent {
        let sequence = self.next_event_sequence;
        self.next_event_sequence += 1;

        FactoryEvent {
            sequence,
            sim_time,
            event,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PlacementError {
    UnknownPart(PartId),
    UnknownConveyor(DeviceId),
    PartNotUnplaced(PartId),
}

struct ExitCandidate {
    part_id: PartId,
    conveyor_id: DeviceId,
    sim_time: Duration,
}

impl Default for Factory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_created_part_has_id_zero() {
        let mut factory = Factory::new();

        assert_eq!(
            factory.create_part().event,
            Event::PartCreated {
                part_id: PartId::new(0)
            }
        );
    }

    #[test]
    fn created_part_at_start_has_zero_simulation_time() {
        let mut factory = Factory::new();

        assert_eq!(factory.create_part().sim_time, Duration::ZERO);
    }

    #[test]
    fn factory_starts_at_zero_simulation_time() {
        assert_eq!(Factory::new().sim_time(), Duration::ZERO);
    }

    #[test]
    fn update_advances_simulation_time_by_its_delta() {
        let mut factory = Factory::new();

        factory.update(Duration::from_millis(100));
        factory.update(Duration::from_millis(250));

        assert_eq!(factory.sim_time(), Duration::from_millis(350));
    }

    #[test]
    fn zero_duration_update_is_a_no_op() {
        let mut factory = Factory::new();

        let events = factory.update(Duration::ZERO);

        assert!(events.is_empty());
        assert_eq!(factory.sim_time(), Duration::ZERO);
    }

    #[test]
    fn created_events_use_current_time_and_sequence_orders_same_time_events() {
        let mut factory = Factory::new();

        let first = factory.create_part();
        let second = factory.create_part();
        factory.update(Duration::from_millis(100));
        let third = factory.create_part();

        assert_eq!(first.sequence, 0);
        assert_eq!(first.sim_time, Duration::ZERO);
        assert_eq!(second.sequence, 1);
        assert_eq!(second.sim_time, Duration::ZERO);
        assert_eq!(third.sequence, 2);
        assert_eq!(third.sim_time, Duration::from_millis(100));
    }

    #[test]
    #[should_panic(expected = "simulation time overflow")]
    fn update_panics_when_simulation_time_overflows() {
        let mut factory = Factory::new();

        factory.update(Duration::MAX);
        factory.update(Duration::from_nanos(1));
    }

    #[test]
    fn identical_operations_produce_identical_observable_results() {
        let mut first_factory = Factory::new();
        let mut second_factory = Factory::new();

        let first_created = first_factory.create_part();
        let second_created = second_factory.create_part();
        let first_update = first_factory.update(Duration::from_millis(100));
        let second_update = second_factory.update(Duration::from_millis(100));
        let first_created_after_update = first_factory.create_part();
        let second_created_after_update = second_factory.create_part();
        let first_second_update = first_factory.update(Duration::from_millis(50));
        let second_second_update = second_factory.update(Duration::from_millis(50));
        let first_third_update = first_factory.update(Duration::from_millis(25));
        let second_third_update = second_factory.update(Duration::from_millis(25));
        let first_final_created = first_factory.create_part();
        let second_final_created = second_factory.create_part();

        assert_eq!(first_created, second_created);
        assert_eq!(first_update, second_update);
        assert_eq!(first_created_after_update, second_created_after_update);
        assert_eq!(first_second_update, second_second_update);
        assert_eq!(first_third_update, second_third_update);
        assert_eq!(first_final_created, second_final_created);
        assert_eq!(first_factory.sim_time(), second_factory.sim_time());
    }

    #[test]
    fn partitioned_time_advancement_reaches_the_same_time() {
        let mut one_update = Factory::new();
        let mut two_updates = Factory::new();

        one_update.update(Duration::from_millis(100));
        two_updates.update(Duration::from_millis(50));
        two_updates.update(Duration::from_millis(50));

        assert_eq!(one_update.sim_time(), two_updates.sim_time());
    }

    #[test]
    fn created_parts_receive_unique_ids() {
        let mut factory = Factory::new();

        let first = factory.create_part();
        let second = factory.create_part();

        let Event::PartCreated {
            part_id: first_part_id,
        } = first.event
        else {
            panic!("create_part must emit PartCreated");
        };
        let Event::PartCreated {
            part_id: second_part_id,
        } = second.event
        else {
            panic!("create_part must emit PartCreated");
        };

        assert_ne!(first_part_id, second_part_id);
    }

    #[test]
    fn event_sequences_increase_monotonically_from_zero() {
        let mut factory = Factory::new();

        let sequences = (0..4)
            .map(|_| factory.create_part().sequence)
            .collect::<Vec<_>>();

        assert_eq!(sequences, [0, 1, 2, 3]);
    }

    #[test]
    fn created_part_is_unplaced_until_placed_on_a_conveyor() {
        let mut factory = Factory::new();
        let part_id = PartId::new(0);

        factory.create_part();

        assert_eq!(factory.part_location(part_id), Some(PartLocation::Unplaced));

        let event = factory
            .place_part_on_conveyor(part_id, ENTRY_CONVEYOR_ID)
            .unwrap();

        assert_eq!(
            event.event,
            Event::PartEnteredConveyor {
                part_id,
                conveyor_id: ENTRY_CONVEYOR_ID,
            }
        );
        assert_eq!(event.sim_time, Duration::ZERO);
        assert_on_conveyor(&factory, part_id, ENTRY_CONVEYOR_ID, 0.0);
    }

    #[test]
    fn placement_rejects_unknown_or_non_unplaced_parts() {
        let mut factory = Factory::new();
        let part_id = PartId::new(0);

        assert_eq!(
            factory.place_part_on_conveyor(part_id, ENTRY_CONVEYOR_ID),
            Err(PlacementError::UnknownPart(part_id))
        );
        assert_eq!(
            factory.place_part_on_conveyor(PartId::new(99), DeviceId::new(99)),
            Err(PlacementError::UnknownConveyor(DeviceId::new(99)))
        );

        factory.create_part();
        factory
            .place_part_on_conveyor(part_id, ENTRY_CONVEYOR_ID)
            .unwrap();

        assert_eq!(
            factory.place_part_on_conveyor(part_id, ENTRY_CONVEYOR_ID),
            Err(PlacementError::PartNotUnplaced(part_id))
        );
    }

    #[test]
    fn placed_part_moves_by_speed_times_elapsed_time() {
        let mut factory = Factory::new();
        let part_id = PartId::new(0);

        factory.create_part();
        factory
            .place_part_on_conveyor(part_id, ENTRY_CONVEYOR_ID)
            .unwrap();
        let events = factory.update(Duration::from_millis(1_500));

        assert!(events.is_empty());
        assert_eq!(factory.sim_time(), Duration::from_millis(1_500));
        assert_on_conveyor(&factory, part_id, ENTRY_CONVEYOR_ID, 3.0);
    }

    #[test]
    fn part_does_not_exit_before_reaching_the_conveyor_end() {
        let mut factory = placed_factory();

        let events = factory.update(Duration::from_millis(4_900));

        assert!(events.is_empty());
        assert_on_conveyor(&factory, PartId::new(0), ENTRY_CONVEYOR_ID, 9.8);
    }

    #[test]
    fn part_exits_at_the_exact_conveyor_boundary() {
        let mut factory = placed_factory();

        let events = factory.update(Duration::from_secs(5));

        assert_eq!(
            factory.part_location(PartId::new(0)),
            Some(PartLocation::Exited)
        );
        assert_eq!(
            events,
            vec![FactoryEvent {
                sequence: 2,
                sim_time: Duration::from_secs(5),
                event: Event::PartExitedConveyor {
                    part_id: PartId::new(0),
                    conveyor_id: ENTRY_CONVEYOR_ID,
                },
            }]
        );
    }

    #[test]
    fn exit_event_uses_crossing_time_inside_a_larger_update() {
        let mut factory = placed_factory();

        let events = factory.update(Duration::from_secs(8));

        assert_eq!(factory.sim_time(), Duration::from_secs(8));
        assert_eq!(events[0].sim_time, Duration::from_secs(5));
        assert_eq!(
            factory.part_location(PartId::new(0)),
            Some(PartLocation::Exited)
        );
    }

    #[test]
    fn partitioned_updates_preserve_exit_lifecycle_and_time() {
        let mut one_update = placed_factory();
        let mut four_updates = placed_factory();

        let one_update_events = one_update.update(Duration::from_secs(8));
        let four_update_events = (0..4)
            .flat_map(|_| four_updates.update(Duration::from_secs(2)))
            .collect::<Vec<_>>();

        assert_eq!(one_update.sim_time(), four_updates.sim_time());
        assert_eq!(
            one_update.part_location(PartId::new(0)),
            four_updates.part_location(PartId::new(0))
        );
        assert_eq!(one_update_events, four_update_events);
    }

    #[test]
    fn non_round_crossing_time_uses_duration_conversion() {
        let conveyor_id = DeviceId::new(7);
        let mut factory = Factory::with_conveyor(Conveyor::new(conveyor_id, 1.0, 3.0));
        let part_id = PartId::new(0);

        factory.create_part();
        factory
            .place_part_on_conveyor(part_id, conveyor_id)
            .unwrap();
        let events = factory.update(Duration::from_secs(1));

        assert_eq!(events[0].sim_time, Duration::from_secs_f64(1.0 / 3.0));
    }

    #[test]
    fn simultaneous_exits_are_ordered_by_part_id() {
        let mut factory = Factory::new();

        factory.create_part();
        factory.create_part();
        factory
            .place_part_on_conveyor(PartId::new(0), ENTRY_CONVEYOR_ID)
            .unwrap();
        factory
            .place_part_on_conveyor(PartId::new(1), ENTRY_CONVEYOR_ID)
            .unwrap();

        let events = factory.update(Duration::from_secs(5));

        assert_eq!(events[0].sim_time, events[1].sim_time);
        assert_eq!(events[0].sequence, 4);
        assert_eq!(events[1].sequence, 5);
        assert_eq!(
            events[0].event,
            Event::PartExitedConveyor {
                part_id: PartId::new(0),
                conveyor_id: ENTRY_CONVEYOR_ID,
            }
        );
        assert_eq!(
            events[1].event,
            Event::PartExitedConveyor {
                part_id: PartId::new(1),
                conveyor_id: ENTRY_CONVEYOR_ID,
            }
        );
    }

    fn placed_factory() -> Factory {
        let mut factory = Factory::new();
        factory.create_part();
        factory
            .place_part_on_conveyor(PartId::new(0), ENTRY_CONVEYOR_ID)
            .unwrap();
        factory
    }

    fn assert_on_conveyor(
        factory: &Factory,
        part_id: PartId,
        conveyor_id: DeviceId,
        expected_position_m: f64,
    ) {
        let Some(PartLocation::OnConveyor {
            conveyor_id: actual_conveyor_id,
            position_m,
        }) = factory.part_location(part_id)
        else {
            panic!("part must be on a conveyor");
        };

        assert_eq!(actual_conveyor_id, conveyor_id);
        assert!((position_m - expected_position_m).abs() < 1e-9);
    }
}
