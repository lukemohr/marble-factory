//! Descriptions of factory devices shared across component boundaries.

use crate::DeviceId;

/// Categorizes a factory device without defining its behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Conveyor,
    Sensor,
    Diverter,
    Bin,
}

/// Identifies and categorizes a factory device without defining its behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceDescriptor {
    pub id: DeviceId,
    pub kind: DeviceKind,
}
