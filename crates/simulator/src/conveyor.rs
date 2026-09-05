use protocol::DeviceId;

/// Internal behavioral configuration and operating state for one conveyor.
pub(super) struct Conveyor {
    id: DeviceId,
    length_m: f64,
    speed_m_per_s: f64,
}

impl Conveyor {
    pub(super) fn new(id: DeviceId, length_m: f64, speed_m_per_s: f64) -> Self {
        assert!(
            length_m.is_finite() && length_m > 0.0,
            "conveyor length must be finite and positive"
        );
        assert!(
            speed_m_per_s.is_finite() && speed_m_per_s > 0.0,
            "conveyor speed must be finite and positive"
        );

        Self {
            id,
            length_m,
            speed_m_per_s,
        }
    }

    pub(super) fn id(&self) -> DeviceId {
        self.id
    }

    pub(super) fn length_m(&self) -> f64 {
        self.length_m
    }

    pub(super) fn speed_m_per_s(&self) -> f64 {
        self.speed_m_per_s
    }
}
