use protocol::{Event, FactoryEvent, PartId};

/// The in-memory state of the simulated factory.
pub struct Factory {
    next_part_id: u64,
    next_event_sequence: u64,
}

impl Factory {
    /// Creates an empty factory whose IDs and event sequences begin at zero.
    pub fn new() -> Self {
        Self {
            next_part_id: 0,
            next_event_sequence: 0,
        }
    }

    /// Creates a part and returns the event describing that creation.
    pub fn create_part(&mut self) -> FactoryEvent {
        let part_id = PartId::new(self.next_part_id);
        self.next_part_id += 1;

        let sequence = self.next_event_sequence;
        self.next_event_sequence += 1;

        FactoryEvent {
            sequence,
            event: Event::PartCreated { part_id },
        }
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
