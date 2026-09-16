use serde_yaml::Value;
#[cfg(target_os = "linux")]
use std::{fs, path::Path, process::Command};

#[test]
fn release_pipeline_parses_and_requires_supply_chain_gate() {
    let config: Value = serde_yaml::from_str(include_str!("../.circleci/config.yml"))
        .expect("CircleCI configuration must be valid YAML");
    let jobs = config["jobs"]
        .as_mapping()
        .expect("CircleCI jobs must be a mapping");
    assert!(jobs.contains_key(Value::from("dependency_security")));
    assert!(jobs.contains_key(Value::from("build_and_test")));
    assert!(jobs.contains_key(Value::from("build_macos")));
    assert!(jobs.contains_key(Value::from("build_windows")));

    let release = &config["workflows"]["ci_cd"]["jobs"];
    let release_jobs = release
        .as_sequence()
        .expect("workflow jobs must be a sequence");
    for required in [
        "dependency_security",
        "build_and_test",
        "build_macos",
        "build_windows",
    ] {
        assert!(
            release_jobs.iter().any(|job| job.get(required).is_some()),
            "workflow must retain {required}"
        );
    }
    let publish = release_jobs
        .iter()
        .find_map(|job| job.get("publish_github_release"))
        .expect("release workflow must include the publisher");
    let requirements = publish["requires"]
        .as_sequence()
        .expect("publisher must declare prerequisites");
    for required in [
        "dependency_security",
        "build_and_test",
        "build_macos",
        "build_windows",
    ] {
        assert!(
            requirements.contains(&Value::from(required)),
            "publisher must require {required}"
        );
    }
    assert_eq!(publish["context"], Value::from("lbc-release"));
    assert_eq!(
        publish["filters"]["branches"]["ignore"],
        Value::from("/.*/")
    );
    assert!(
        publish["filters"]["tags"]["only"]
            .as_str()
            .is_some_and(|filter| filter.contains("v[0-9]+")),
        "release publication must remain tag-only"
    );

    for platform in ["build_macos", "build_windows"] {
        let workflow_job = release_jobs
            .iter()
            .find_map(|job| job.get(platform))
            .expect("platform job must be in the workflow");
        assert!(
            workflow_job["filters"].get("branches").is_none(),
            "{platform} must run on release-candidate branches before tagging"
        );
        assert!(workflow_job["filters"]["tags"]["only"].is_string());
    }

    let dependency_steps = jobs[&Value::from("dependency_security")]["steps"]
        .as_sequence()
        .expect("dependency security steps must be a sequence");
    assert!(dependency_steps.iter().any(|step| {
        step["run"]["command"]
            .as_str()
            .is_some_and(|command| command.contains("test-release-scripts.sh"))
    }));
    let dependency_commands = dependency_steps
        .iter()
        .filter_map(|step| step["run"]["command"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for required in [
        "cargo audit",
        "cargo deny check",
        "cargo cyclonedx",
        "test-release-scripts.sh",
    ] {
        assert!(
            dependency_commands.contains(required),
            "dependency security must execute {required}"
        );
    }

    let publish_steps = jobs[&Value::from("publish_github_release")]["steps"]
        .as_sequence()
        .expect("publisher steps must be a sequence");
    let publish_commands = publish_steps
        .iter()
        .filter_map(|step| step["run"]["command"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for required in [
        "generate-provenance.sh",
        "sign-release-assets.sh",
        "publish-github-release.sh",
    ] {
        assert!(
            publish_commands.contains(required),
            "publisher must execute {required}"
        );
    }

    for platform in ["build_and_test", "build_macos", "build_windows"] {
        let serialized = serde_yaml::to_string(&jobs[&Value::from(platform)]).unwrap();
        let lowercase = serialized.to_ascii_lowercase();
        assert!(
            serialized.contains("CIRCLE_TAG")
                && (lowercase.contains("package_version") || lowercase.contains("packageversion")),
            "{platform} must enforce Cargo version/tag consistency"
        );
    }

    let publisher = include_str!("../.circleci/publish-github-release.sh");
    assert!(publisher.contains("verify-release-assets.sh"));
    assert!(publisher.contains("draft: true"));
    assert!(publisher.contains("Refusing to replace assets on an already-published release"));
}

#[cfg(target_os = "linux")]
fn write_release_inputs(dist: &Path) {
    fs::create_dir_all(dist).unwrap();
    let version = env!("CARGO_PKG_VERSION");
    for archive in [
        format!("lbc-{version}-x86_64-unknown-linux-gnu.tar.gz"),
        format!("lbc-{version}-universal-apple-darwin.tar.gz"),
        format!("lbc-{version}-x86_64-pc-windows-msvc.zip"),
    ] {
        fs::write(dist.join(&archive), format!("fixture bytes for {archive}")).unwrap();
        let digest = Command::new("sha256sum")
            .arg(dist.join(&archive))
            .output()
            .unwrap();
        assert!(digest.status.success());
        let digest = String::from_utf8(digest.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .to_owned();
        fs::write(
            dist.join(format!("{archive}.sha256")),
            format!("{digest}  {archive}\n"),
        )
        .unwrap();
    }
    fs::write(
        dist.join(format!("lbc-{version}.cdx.json")),
        r#"{"bomFormat":"CycloneDX","components":[{"name":"fixture","version":"1.0.0","licenses":[{"license":{"id":"MIT"}}],"hashes":[{"alg":"SHA-256","content":"00"}]}]}"#,
    )
    .unwrap();
}

#[cfg(target_os = "linux")]
fn provenance_command(dist: &Path) -> std::process::Output {
    Command::new("bash")
        .arg(".circleci/generate-provenance.sh")
        .arg(dist)
        .env("CIRCLE_SHA1", "1111111111111111111111111111111111111111")
        .env("CIRCLE_PROJECT_USERNAME", "fixture-owner")
        .env("CIRCLE_PROJECT_REPONAME", "fixture-repository")
        .env("CIRCLE_WORKFLOW_ID", "fixture-workflow")
        .env("CIRCLE_WORKFLOW_NAME", "ci_cd")
        .env("CIRCLE_JOB", "publish_github_release")
        .env(
            "CIRCLE_BUILD_URL",
            "https://circleci.example.invalid/build/1",
        )
        .output()
        .unwrap()
}

#[test]
#[cfg(target_os = "linux")]
fn provenance_requires_every_final_input_and_valid_checksums() {
    let root = tempfile::tempdir().unwrap();
    let dist = root.path().join("dist");
    write_release_inputs(&dist);
    fs::remove_file(dist.join(format!("lbc-{}.cdx.json", env!("CARGO_PKG_VERSION")))).unwrap();
    let missing_sbom = provenance_command(&dist);
    assert!(!missing_sbom.status.success());
    assert!(String::from_utf8_lossy(&missing_sbom.stderr).contains("missing or unsafe"));

    write_release_inputs(&dist);
    let linux_checksum = dist.join(format!(
        "lbc-{}-x86_64-unknown-linux-gnu.tar.gz.sha256",
        env!("CARGO_PKG_VERSION")
    ));
    fs::write(&linux_checksum, "0  wrong\n").unwrap();
    let bad_checksum = provenance_command(&dist);
    assert!(!bad_checksum.status.success());
    assert!(String::from_utf8_lossy(&bad_checksum.stderr).contains("Checksum file"));
}

#[test]
#[cfg(target_os = "linux")]
fn provenance_covers_the_exact_archives_checksums_and_sbom() {
    let root = tempfile::tempdir().unwrap();
    let dist = root.path().join("dist");
    write_release_inputs(&dist);
    let output = provenance_command(&dist);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let path = dist.join(format!("lbc-{}.provenance.json", env!("CARGO_PKG_VERSION")));
    let provenance: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let subjects = provenance["subject"].as_array().unwrap();
    assert_eq!(subjects.len(), 7);
    assert!(subjects.iter().any(|subject| {
        subject["name"]
            .as_str()
            .is_some_and(|name| name.ends_with(".sha256"))
    }));
    assert_eq!(
        provenance["predicate"]["buildDefinition"]["resolvedDependencies"][0]["digest"]["gitCommit"],
        "1111111111111111111111111111111111111111"
    );
    assert_eq!(
        provenance["predicate"]["buildDefinition"]["resolvedDependencies"][0]["uri"],
        "git+https://github.com/fixture-owner/fixture-repository"
    );
    assert_eq!(
        provenance["predicate"]["runDetails"]["metadata"]["jobName"],
        "publish_github_release"
    );
    assert_eq!(
        provenance["predicate"]["runDetails"]["metadata"]["buildUrl"],
        "https://circleci.example.invalid/build/1"
    );
}

#[test]
#[cfg(target_os = "linux")]
fn release_scripts_reject_mismatched_tags_before_publication() {
    let root = tempfile::tempdir().unwrap();
    let output = Command::new("bash")
        .arg(".circleci/generate-provenance.sh")
        .arg(root.path())
        .env("CIRCLE_TAG", "v9.9.9")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("does not match"));
}
