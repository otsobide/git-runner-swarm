//! The ports the application works through, each implemented by adapters
//! outside it.

mod runner_image;
mod runner_runtime;

pub use runner_image::{EnvironmentVariable, LaunchSpec, RestartPolicy, RunnerImage};
pub use runner_runtime::{RunnerRuntime, RuntimeError};
