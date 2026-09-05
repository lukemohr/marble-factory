//! Events emitted by the factory.

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
    pub event: Event,
}
