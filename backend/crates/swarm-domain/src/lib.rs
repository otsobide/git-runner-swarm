//! The domain: what a runner is, independent of the forge it serves and of
//! the container engine it runs on.
//!
//! Every value a runner is made of is a value object: it is validated once,
//! when it is built, so a value of its type is always a valid one and code
//! holding it never checks it again.

mod error;
mod forge;
mod forge_address;
mod image_reference;
mod owner_name;
mod registration_token;
mod repository_name;
mod runner;
mod runner_definition;
mod runner_id;
mod runner_label;
mod runner_lifecycle;
mod runner_name;
mod runner_scope;
mod runner_status;
mod validate;

pub use error::InvalidValue;
pub use forge::Forge;
pub use forge_address::ForgeAddress;
pub use image_reference::ImageReference;
pub use owner_name::OwnerName;
pub use registration_token::RegistrationToken;
pub use repository_name::RepositoryName;
pub use runner::Runner;
pub use runner_definition::RunnerDefinition;
pub use runner_id::RunnerId;
pub use runner_label::{RunnerLabel, RunnerLabels};
pub use runner_lifecycle::RunnerLifecycle;
pub use runner_name::RunnerName;
pub use runner_scope::{RunnerScope, ScopeKind};
pub use runner_status::RunnerStatus;
