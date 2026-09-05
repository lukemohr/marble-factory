//! Strongly typed identifiers used by factory components.

/// Identifies one factory device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceId(u64);

impl DeviceId {
    /// Creates an identifier from its deterministic numeric value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

/// Identifies one part in the factory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PartId(u64);

impl PartId {
    /// Creates an identifier from its deterministic numeric value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}
