//! Data contracts shared between factory components.

pub mod devices;
pub mod ids;

pub use devices::{DeviceDescriptor, DeviceKind};
pub use ids::{DeviceId, PartId};
