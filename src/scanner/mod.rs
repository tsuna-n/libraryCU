pub mod detector;
pub mod files;
pub mod ignore;
pub mod project;

pub use detector::{detect_project, find_project_root};
pub use files::{ScanEntry, ScanReport, scan_project};
pub use project::ProjectInfo;
