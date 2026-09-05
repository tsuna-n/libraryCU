pub mod document;
pub mod index;
pub mod loader;
pub mod packages;
pub mod retrieval;
pub mod search;
pub mod storage;

pub use document::{KnowledgeDocument, KnowledgeMetadata};
pub use index::{KnowledgeIndex, SearchResult};
pub use loader::{InvalidDocument, LoadReport, load_all_documents, load_documents};
pub use packages::{
    InstalledPackage, PackageManifest, data_dir, install_package, list_packages, remove_package,
};
pub use retrieval::{RetrievalReport, retrieve};
pub use search::search;
pub use storage::{
    AddEntry, EditEntry, add_entry, edit_entry, inspect_entry, notes_dir, resolve_entry,
};
