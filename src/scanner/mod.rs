pub mod detector;
pub mod files;
pub mod ignore;
pub mod project;

pub use context::{EvidenceExcerpt, collect_diagnostic_evidence};
pub use detector::{detect_project, find_project_root};
pub use files::{ScanEntry, ScanReport, scan_project};
pub use project::ProjectInfo;
pub mod context;
