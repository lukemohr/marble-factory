use protocol::{DeviceId, PartId};

/// Simulator-owned lifecycle state for one part.
pub(super) struct Part {
    pub(super) id: PartId,
    pub(super) location: PartLocation,
}

impl Part {
    pub(super) fn new(id: PartId) -> Self {
        Self {
            id,
            location: PartLocation::Unplaced,
        }
    }
}

/// Read-only physical location and lifecycle state for a part.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum PartLocation {
    Unplaced,
    OnConveyor {
        conveyor_id: DeviceId,
        position_m: f64,
    },
    Exited,
}
