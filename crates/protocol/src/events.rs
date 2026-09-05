//! Events emitted by the factory.

use std::time::Duration;

use crate::PartId;

/// A fact that occurred in the factory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    PartCreated { part_id: PartId },
}

/// An event with metadata that applies to every factory event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactoryEvent {
    pub sequence: u64,
    /// Elapsed simulation time when this event occurred.
    pub sim_time: Duration,
    pub event: Event,
}
