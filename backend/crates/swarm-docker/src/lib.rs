//! Runs runners as Docker containers.
//!
//! A runner's definition is kept on its container, as labels, so Docker is
//! the one place runners are kept: the runners listed are the containers
//! there are.

mod engine_socket;
mod gpu_devices;
mod labels;
mod runtime;

pub use engine_socket::{EngineSocket, local_socket, socket_group};
pub use gpu_devices::GpuDevices;
pub use runtime::DockerRuntime;
