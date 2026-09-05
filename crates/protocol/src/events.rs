//! Events emitted by the factory.

use crate::PartId;

/// A fact that occurred in the factory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    PartCreated { part_id: PartId },
}
