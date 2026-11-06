//! The domain: what a runner is, independent of the forge it serves and of
//! the container engine it runs on.
//!
//! Every value a runner is made of is a value object: it is validated once,
//! when it is built, so a value of its type is always a valid one and code
//! holding it never checks it again.

mod error;
mod forge;

pub use error::InvalidValue;
pub use forge::Forge;
