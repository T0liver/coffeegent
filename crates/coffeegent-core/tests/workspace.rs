//! M0 workspace checks for the core crate.

#[test]
fn version_is_semver_like() {
    let v = coffeegent_core::VERSION;
    assert!(
        !v.is_empty() && v.chars().next().is_some_and(|c| c.is_ascii_digit()),
        "VERSION should be set: {v}"
    );
}
