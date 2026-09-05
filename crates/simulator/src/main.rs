use protocol::{Event, FactoryEvent, PartId};
use std::time::Duration;

/// The in-memory state of the simulated factory.
// The binary gains its runtime loop in a later chunk; tests exercise this state today.
#[cfg_attr(not(test), allow(dead_code))]
struct Factory {
    sim_time: Duration,
    next_part_id: u64,
    next_event_sequence: u64,
}

#[cfg_attr(not(test), allow(dead_code))]
impl Factory {
    /// Creates an empty factory whose IDs and event sequences begin at zero.
    fn new() -> Self {
        Self {
            sim_time: Duration::ZERO,
            next_part_id: 0,
            next_event_sequence: 0,
        }
    }

    /// Creates a part and returns the event describing that creation.
    fn create_part(&mut self) -> FactoryEvent {
        let part_id = PartId::new(self.next_part_id);
        self.next_part_id += 1;

        let sequence = self.next_event_sequence;
        self.next_event_sequence += 1;

        FactoryEvent {
            sequence,
            sim_time: self.sim_time,
            event: Event::PartCreated { part_id },
        }
    }

    /// Returns the factory's current elapsed simulation time.
    fn sim_time(&self) -> Duration {
        self.sim_time
    }

    /// Advances simulation time and returns any events caused by that interval.
    fn update(&mut self, dt: Duration) -> Vec<FactoryEvent> {
        self.sim_time = self
            .sim_time
            .checked_add(dt)
            .expect("simulation time overflow");

        Vec::new()
    }
}

impl Default for Factory {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {}

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
        } = first.event;
        let Event::PartCreated {
            part_id: second_part_id,
        } = second.event;

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
}
