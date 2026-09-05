//! Descriptions of factory devices shared across component boundaries.

/// Categorizes a factory device without defining its behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Conveyor,
    Sensor,
    Diverter,
    Bin,
}
