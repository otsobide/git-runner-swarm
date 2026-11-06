//! The application: what the panel does with runners (create them, list
//! them, start, stop and delete them), written against ports, the traits
//! in [`ports`], which adapters implement.
//!
//! Nothing here knows which container engine runs a runner, or which image
//! it runs: an adapter of [`ports::RunnerImage`] knows how to start one
//! image, and an adapter of the runtime port knows one engine.

mod images;
pub mod ports;
#[cfg(test)]
mod testing;

pub use images::RunnerImages;
