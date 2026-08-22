pub mod detector;
pub mod project;

pub use detector::{detect_project, find_project_root};
pub use project::ProjectInfo;
