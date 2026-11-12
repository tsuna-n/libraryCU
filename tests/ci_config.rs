use serde_yaml::Value;
#[cfg(target_os = "linux")]
use std::path::Path;
#[cfg(unix)]
use std::{fs, process::Command};

#[test]
#[cfg(unix)]
fn unix_release_archive_installs_and_uninstalls_without_removing_user_files() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("package");
    fs::create_dir(&package).unwrap();
    fs::copy("install.sh", package.join("install.sh")).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_lbc"), package.join("lbc")).unwrap();
    let archive = root.path().join("release.tar.gz");
    assert!(
        Command::new("tar")
            .args(["-czf", archive.to_str().unwrap(), "-C"])
            .arg(root.path())
            .arg("package")
            .status()
            .unwrap()
            .success()
    );
    let extracted = root.path().join("extracted");
    fs::create_dir(&extracted).unwrap();
    assert!(
        Command::new("tar")
            .arg("-xzf")
            .arg(archive)
            .arg("-C")
            .arg(&extracted)
            .status()
            .unwrap()
            .success()
    );
    let installer = extracted.join("package/install.sh");
    let prefix = root.path().join("prefix with spaces");
    fs::create_dir(&prefix).unwrap();
    fs::write(prefix.join("user-note.md"), "keep my knowledge").unwrap();
    let install = Command::new("bash")
        .arg(&installer)
        .arg("--prefix")
        .arg(&prefix)
        .output()
        .unwrap();
    assert!(install.status.success(), "{install:?}");
    let version = Command::new(prefix.join("lbc"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).unwrap().trim(),
        format!("lbc {}", env!("CARGO_PKG_VERSION"))
    );
    for _ in 0..2 {
        assert!(
            Command::new("bash")
                .arg(&installer)
                .arg("--prefix")
                .arg(&prefix)
                .arg("--uninstall")
                .output()
                .unwrap()
                .status
                .success()
        );
        assert!(!prefix.join("lbc").exists());
        assert_eq!(
            fs::read_to_string(prefix.join("user-note.md")).unwrap(),
            "keep my knowledge"
        );
    }
}

#[test]
#[cfg(unix)]
fn unix_bundled_installer_rejects_a_binary_that_fails_version_verification() {
    let root = tempfile::tempdir().unwrap();
    fs::copy("install.sh", root.path().join("install.sh")).unwrap();
    for binary in [
        "#!/bin/sh\nexit 37\n",
        "#!/bin/sh\necho unrelated-command\n",
    ] {
        fs::write(root.path().join("lbc"), binary).unwrap();
        let output = Command::new("bash")
            .arg(root.path().join("install.sh"))
            .arg("--prefix")
            .arg(root.path().join("prefix"))
            .output()
            .unwrap();
        assert!(!output.status.success(), "{output:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("Installation verification failed")
        );
    }
}

#[test]
fn native_candidates_test_installers_from_the_extracted_release_archives() {
    let config: Value = serde_yaml::from_str(include_str!("../.circleci/config.yml")).unwrap();
    for (job, checks) in [
        (
            "build_and_test",
            vec![
                "tar -xzf \"dist/${archive_name}\"",
                "archive/$(basename \"${package_root}\")/install.sh",
                "prefix/lbc\" --version",
                "--uninstall",
                "test ! -e \"${install_test_root}/prefix/lbc\"",
            ],
        ),
        (
            "build_macos",
            vec![
                "tar -xzf \"dist/${package_name}.tar.gz\"",
                "archive/${package_name}/install.sh",
                "prefix/lbc\" --version",
                "--uninstall",
                "test ! -e \"${install_test_root}/prefix/lbc\"",
            ],
        ),
        (
            "build_windows",
            vec![
                "Expand-Archive -LiteralPath $archivePath",
                "$extractRoot \"$packageName\\install.ps1\"",
                "& $installedBinary --version",
                "-Prefix $prefix -Uninstall",
                "Test-Path -LiteralPath $installedBinary",
                "$userPath -notcontains $prefix",
                "$userPath -contains $prefix",
            ],
        ),
    ] {
        let commands = config["jobs"][job]["steps"]
            .as_sequence()
            .unwrap()
            .iter()
            .filter_map(|step| step["run"]["command"].as_str())
            .collect::<Vec<_>>()
            .join("\n");
        for check in checks {
            assert!(commands.contains(check), "{job} must retain {check}");
        }
    }
}

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
    assert_eq!(
        publish["context"],
        Value::Sequence(vec![
            Value::from("lbc-release"),
            Value::from("lbc-release-identity")
        ])
    );
    for required in [
        "release_source_gate",
        "sign_macos_release",
        "sign_windows_release",
        "approve_production_release",
    ] {
        assert!(requirements.contains(&Value::from(required)));
    }
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
    let version = env!("CARGO_PKG_VERSION");
    let output = Command::new("python3")
        .arg(".circleci/create-release-fixture.py")
        .arg(dist)
        .arg(version)
        .arg("1111111111111111111111111111111111111111")
        .arg("fixture-workflow")
        .env("LBC_WINDOWS_CERT_THUMBPRINT", "A".repeat(40))
        .env("LBC_MACOS_IDENTITY_SHA1", "B".repeat(40))
        .env("LBC_MACOS_TEAM_ID", "FIXTURE123")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(target_os = "linux")]
fn provenance_command(dist: &Path) -> std::process::Output {
    Command::new("bash")
        .arg(".circleci/generate-provenance.sh")
        .arg(dist)
        .env("CIRCLE_SHA1", "1111111111111111111111111111111111111111")
        .env("CIRCLE_PROJECT_USERNAME", "tsuna-n")
        .env("CIRCLE_PROJECT_REPONAME", "libraryCU")
        .env("LBC_WINDOWS_CERT_THUMBPRINT", "A".repeat(40))
        .env("LBC_MACOS_IDENTITY_SHA1", "B".repeat(40))
        .env("LBC_MACOS_TEAM_ID", "FIXTURE123")
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
    assert_eq!(subjects.len(), 11);
    let parameters = &provenance["predicate"]["buildDefinition"]["externalParameters"];
    assert_eq!(parameters["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(parameters["tag"], format!("v{}", env!("CARGO_PKG_VERSION")));
    assert_eq!(
        parameters["ref"],
        format!("refs/tags/v{}", env!("CARGO_PKG_VERSION"))
    );
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
        "git+https://github.com/tsuna-n/libraryCU"
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

#[test]
fn native_signing_jobs_are_tag_only_and_credential_scoped() {
    let config: Value = serde_yaml::from_str(include_str!("../.circleci/config.yml")).unwrap();
    let workflow = config["workflows"]["ci_cd"]["jobs"].as_sequence().unwrap();
    let find = |name: &str| workflow.iter().find_map(|job| job.get(name)).unwrap();
    for candidate in [
        "dependency_security",
        "build_and_test",
        "build_macos",
        "build_windows",
    ] {
        assert!(
            find(candidate).get("context").is_none(),
            "candidate must not receive release credentials"
        );
    }
    for (job, context) in [
        ("release_source_gate", "lbc-release-identity"),
        ("sign_macos_release", "lbc-release-macos"),
        ("sign_windows_release", "lbc-release-windows"),
    ] {
        assert_eq!(find(job)["context"], Value::from(context));
        assert_eq!(
            find(job)["filters"]["branches"]["ignore"],
            Value::from("/.*/")
        );
        assert!(find(job)["filters"]["tags"]["only"].is_string());
    }
    for native in ["sign_macos_release", "sign_windows_release"] {
        assert_eq!(
            find(native)["requires"],
            Value::Sequence(vec![Value::from("release_source_gate")])
        );
        let steps = config["jobs"][native]["steps"].as_sequence().unwrap();
        let sign_index = steps
            .iter()
            .position(|step| step["run"]["name"].is_string())
            .unwrap();
        let artifact_index = steps
            .iter()
            .position(|step| step["store_artifacts"]["path"].as_str() == Some("production-dist"))
            .expect("owner must be able to download staged native evidence before approval");
        assert!(artifact_index > sign_index);
    }
    for gate in [
        "dependency_security",
        "build_and_test",
        "build_macos",
        "build_windows",
    ] {
        assert!(
            find("release_source_gate")["requires"]
                .as_sequence()
                .unwrap()
                .contains(&Value::from(gate))
        );
    }
    assert_eq!(
        find("approve_production_release")["type"],
        Value::from("approval")
    );
    assert_eq!(
        find("approve_production_release")["filters"]["branches"]["ignore"],
        Value::from("/.*/")
    );
}

#[test]
#[cfg(target_os = "linux")]
fn production_manifest_is_exact_and_fully_signed() {
    let output = Command::new("bash")
        .arg("-c")
        .arg("source .circleci/release-common.sh; set_release_assets \"$(release_version)\"; printf '%s\\n' \"${RELEASE_ASSETS[@]}\"")
        .output().unwrap();
    assert!(output.status.success());
    let names = String::from_utf8(output.stdout).unwrap();
    let names: Vec<_> = names.lines().collect();
    assert_eq!(names.len(), 25);
    assert_eq!(
        names
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        25
    );
    for name in names.iter().filter(|name| !name.ends_with(".asc")) {
        assert!(
            names.contains(&format!("{name}.asc").as_str()),
            "every production input must be signed"
        );
    }
    assert!(names.iter().any(|name| name.ends_with(".dmg")));
    assert!(
        names
            .iter()
            .any(|name| name.ends_with(".macos-signing.json"))
    );
    assert!(
        names
            .iter()
            .any(|name| name.ends_with(".windows-signing.json"))
    );
    assert!(names.contains(&"lbc-release-signing-key.asc"));
}
