//! Data contracts shared between factory components.

pub mod devices;
pub mod events;
pub mod ids;

pub use devices::{DeviceDescriptor, DeviceKind};
pub use events::{Event, FactoryEvent};
pub use ids::{DeviceId, PartId};
