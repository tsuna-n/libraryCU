use std::path::Path;

pub const GENERATED_DIRECTORIES: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    ".next",
    "coverage",
    "vendor",
    "__pycache__",
    ".venv",
    "venv",
];

pub fn should_ignore_directory(path: &Path, ignore_hidden: bool) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    GENERATED_DIRECTORIES.contains(&name) || (ignore_hidden && name.starts_with('.'))
}
