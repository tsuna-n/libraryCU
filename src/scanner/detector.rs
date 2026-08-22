use std::{fs, path::Path};

use anyhow::{Context, Result};

use super::project::ProjectInfo;

const ROOT_MARKERS: &[&str] = &[
    ".git",
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
    "requirements.txt",
    "go.mod",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "Dockerfile",
    "AGENT.md",
    "README.md",
];

pub fn find_project_root(start: &Path) -> Result<std::path::PathBuf> {
    let start = start
        .canonicalize()
        .with_context(|| format!("project path does not exist: {}", start.display()))?;
    let mut current = if start.is_file() {
        start.parent().unwrap_or(&start).to_path_buf()
    } else {
        start
    };
    let fallback = current.clone();

    loop {
        if ROOT_MARKERS
            .iter()
            .any(|marker| current.join(marker).exists())
        {
            return Ok(current);
        }
        if !current.pop() {
            return Ok(fallback);
        }
    }
}

pub fn detect_project(root: &Path) -> Result<ProjectInfo> {
    let root = find_project_root(root)?;
    let mut project = ProjectInfo {
        root: root.clone(),
        ..ProjectInfo::default()
    };

    if root.join("Cargo.toml").is_file() {
        push_unique(&mut project.languages, "Rust");
        push_unique(&mut project.build_systems, "Cargo");
        detect_rust_frameworks(&root, &mut project);
    }
    if root.join("package.json").is_file() {
        push_unique(&mut project.languages, "Node.js");
        detect_node_tooling(&root, &mut project);
        detect_node_frameworks(&root, &mut project);
    }
    if root.join("tsconfig.json").is_file() {
        push_unique(&mut project.languages, "TypeScript");
    }
    if root.join("pyproject.toml").is_file() || root.join("requirements.txt").is_file() {
        push_unique(&mut project.languages, "Python");
        if root.join("pyproject.toml").is_file() {
            push_unique(&mut project.build_systems, "Pyproject");
        }
    }
    if root.join("go.mod").is_file() {
        push_unique(&mut project.languages, "Go");
        push_unique(&mut project.build_systems, "Go modules");
    }
    if root.join("pom.xml").is_file() {
        push_unique(&mut project.languages, "Java");
        push_unique(&mut project.build_systems, "Maven");
    }
    if root.join("build.gradle").is_file() || root.join("build.gradle.kts").is_file() {
        push_unique(&mut project.languages, "Java / Kotlin");
        push_unique(&mut project.build_systems, "Gradle");
    }
    if root.join("Dockerfile").is_file() {
        push_unique(&mut project.containers, "Docker");
    }
    if root.join("docker-compose.yml").is_file()
        || root.join("docker-compose.yaml").is_file()
        || root.join("compose.yml").is_file()
        || root.join("compose.yaml").is_file()
    {
        push_unique(&mut project.containers, "Docker Compose");
    }

    for directory in ["src", "app", "lib"] {
        if root.join(directory).is_dir() {
            project.source_directories.push(directory.into());
        }
    }
    if root.join("migrations").is_dir() {
        project
            .additional
            .push("SQL migrations detected".to_owned());
    }

    Ok(project)
}

fn detect_node_tooling(root: &Path, project: &mut ProjectInfo) {
    if root.join("pnpm-lock.yaml").is_file() {
        push_unique(&mut project.build_systems, "pnpm");
    } else if root.join("yarn.lock").is_file() {
        push_unique(&mut project.build_systems, "Yarn");
    } else if root.join("package-lock.json").is_file() {
        push_unique(&mut project.build_systems, "npm");
    } else {
        push_unique(&mut project.build_systems, "Node package scripts");
    }
}

fn detect_node_frameworks(root: &Path, project: &mut ProjectInfo) {
    let Some(value) = read_json_value(&root.join("package.json")) else {
        return;
    };
    let dependencies = ["dependencies", "devDependencies"];
    for (package, framework) in [
        ("react", "React"),
        ("next", "Next.js"),
        ("vue", "Vue"),
        ("svelte", "Svelte"),
        ("express", "Express"),
    ] {
        if dependencies.iter().any(|section| {
            value
                .get(section)
                .and_then(|entry| entry.get(package))
                .is_some()
        }) {
            push_unique(&mut project.frameworks, framework);
        }
    }
}

fn detect_rust_frameworks(root: &Path, project: &mut ProjectInfo) {
    let path = root.join("Cargo.toml");
    let Ok(content) = read_small_text(&path) else {
        return;
    };
    let Ok(value) = toml::from_str::<toml::Value>(&content) else {
        return;
    };
    for (package, framework) in [
        ("axum", "Axum"),
        ("actix-web", "Actix Web"),
        ("rocket", "Rocket"),
        ("tokio", "Tokio"),
    ] {
        if value
            .get("dependencies")
            .and_then(|dependencies| dependencies.get(package))
            .is_some()
        {
            push_unique(&mut project.frameworks, framework);
        }
    }
}

fn read_json_value(path: &Path) -> Option<serde_json::Value> {
    let content = read_small_text(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn read_small_text(path: &Path) -> Result<String> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > 256 * 1024 {
        anyhow::bail!("metadata file is larger than 256 KB");
    }
    fs::read_to_string(path).map_err(Into::into)
}

fn push_unique(items: &mut Vec<String>, value: &str) {
    if !items.iter().any(|item| item == value) {
        items.push(value.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn project_dir(name: &str) -> Result<std::path::PathBuf> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!("lcu-{name}-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    #[test]
    fn detects_a_multi_stack_project() -> Result<()> {
        let root = project_dir("multi-stack")?;
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"api\"\nversion = \"0.1.0\"\n",
        )?;
        fs::write(
            root.join("package.json"),
            r#"{"dependencies":{"react":"latest"}}"#,
        )?;
        fs::write(root.join("tsconfig.json"), "{}")?;
        fs::write(root.join("compose.yml"), "services: {}")?;
        fs::create_dir(root.join("src"))?;
        fs::create_dir(root.join("migrations"))?;

        let project = detect_project(&root)?;
        assert_eq!(project.languages, ["Rust", "Node.js", "TypeScript"]);
        assert!(project.build_systems.contains(&"Cargo".to_owned()));
        assert!(project.containers.contains(&"Docker Compose".to_owned()));
        assert!(project.frameworks.contains(&"React".to_owned()));
        assert_eq!(project.additional, ["SQL migrations detected"]);
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn finds_root_from_nested_source_directory() -> Result<()> {
        let root = project_dir("nested-root")?;
        let nested = root.join("src/deep");
        fs::create_dir_all(&nested)?;
        fs::write(root.join("Cargo.toml"), "[package]")?;
        assert_eq!(find_project_root(&nested)?, root);
        fs::remove_dir_all(root)?;
        Ok(())
    }
}
