pub mod document;
pub mod index;
pub mod loader;
pub mod packages;
pub mod search;

pub use document::{KnowledgeDocument, KnowledgeMetadata};
pub use index::{KnowledgeIndex, SearchResult};
pub use loader::load_documents;
pub use packages::{
    InstalledPackage, PackageManifest, data_dir, install_package, list_packages, remove_package,
};
pub use search::search;
