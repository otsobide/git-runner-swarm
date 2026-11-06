//! Runner image adapters for GitHub: how to start a container of an image
//! so the GitHub Actions runner in it registers and takes jobs.

mod official;

pub use official::GitHubOfficialImage;
