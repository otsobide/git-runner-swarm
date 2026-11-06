//! The ports the application works through, each implemented by adapters
//! outside it.

mod runner_image;

pub use runner_image::{EnvironmentVariable, LaunchSpec, RestartPolicy, RunnerImage};
