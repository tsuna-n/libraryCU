use serde_yaml::Value;

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
}
