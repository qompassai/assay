use assay::{launch_diff, probe};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

fn check(mode: &str) -> Result<(), launch_diff::Error> {
    let stub = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/profile_stub.py"
    );
    let core = probe::run(&["python3", stub, mode], &[], Duration::from_secs(3)).unwrap();
    assert_eq!(core.server_info["name"], "profile-stub");
    let full = probe::run(
        &["python3", stub, mode, "full"],
        &[],
        Duration::from_secs(3),
    )
    .unwrap();
    let expected = BTreeMap::from([
        ("core", BTreeSet::from(["core_tool".into()])),
        (
            "full",
            BTreeSet::from(["core_tool".into(), "full_tool".into()]),
        ),
    ]);
    launch_diff::assert_profiles(&BTreeMap::from([("core", core), ("full", full)]), &expected)
}

#[test]
fn correct_launch_profiles_pass() {
    check("fixed").unwrap();
}

#[test]
fn over_listing_is_a_profile_failure() {
    assert_eq!(
        check("buggy"),
        Err(launch_diff::Error::WrongTools {
            profile: "core".into(),
            missing: BTreeSet::new(),
            unexpected: BTreeSet::from(["full_tool".into()]),
        })
    );
}
