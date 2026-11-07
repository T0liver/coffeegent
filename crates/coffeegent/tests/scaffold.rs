//! Repository scaffold checks (M0).

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

#[test]
fn inhibit_spike_reference_exists() {
    let spike = repo_root().join("docs/reference/inhibit_spike.rs");
    let text = std::fs::read_to_string(&spike).expect("inhibit spike must be present");
    assert!(
        text.contains("sleep:idle:handle-lid-switch"),
        "spike should document the observed lock string"
    );
}

#[test]
fn confirmed_rulings_documented() {
    let rulings = repo_root().join("docs/rulings.md");
    let text = std::fs::read_to_string(&rulings).expect("rulings baseline must exist");
    for needle in ["state.json", "min_hold_secs", "`toggle`", "always exits 0"] {
        assert!(text.contains(needle), "rulings must mention {needle}");
    }
}
