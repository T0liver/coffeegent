//! The core crate parses values it is given. It does not gain I/O crates.

use std::fs;
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn cargo_manifest_has_no_io_dependencies() {
    let text = fs::read_to_string(manifest_dir().join("Cargo.toml")).expect("manifest");
    let deps = text
        .split_once("[dependencies]")
        .expect("dependencies")
        .1
        .split_once("[lints]")
        .expect("lints")
        .0;
    for forbidden in [
        "tokio",
        "zbus",
        "serde_json",
        "nix",
        "notify ",
        "reqwest",
        "libc",
    ] {
        assert!(
            !deps.contains(forbidden),
            "coffeegent-core must not depend on {forbidden}"
        );
    }
    for allowed in ["humantime", "thiserror", "toml"] {
        assert!(deps.contains(allowed), "expected parser dep {allowed}");
    }
}

#[test]
fn sources_do_not_perform_io() {
    let src = manifest_dir().join("src");
    for entry in fs::read_dir(src).expect("src") {
        let path = entry.expect("entry").path();
        let text = fs::read_to_string(&path).expect("source");
        for forbidden in [
            "std::fs",
            "std::net",
            "std::os::",
            "tokio",
            "File::",
            "TcpStream",
            "UdpSocket",
        ] {
            assert!(
                !text.contains(forbidden),
                "{} contains {forbidden}",
                path.display()
            );
        }
    }
}
