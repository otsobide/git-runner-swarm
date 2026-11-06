//! Runs runners as Docker containers.
//!
//! A runner's definition is kept on its container, as labels, so Docker is
//! the one place runners are kept: the runners listed are the containers
//! there are.

mod labels;
mod runtime;

pub use runtime::DockerRuntime;
