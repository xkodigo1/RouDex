//! The workspace version in `Cargo.toml` is the only source of truth. The
//! desktop app takes it from Cargo and the UI package must repeat it.

use serde_json::Value;
use std::path::Path;

fn json(relative: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

#[test]
fn the_ui_package_has_the_workspace_version() {
    assert_eq!(
        json("../ui/package.json")["version"],
        env!("CARGO_PKG_VERSION")
    );
}

#[test]
fn the_desktop_app_takes_its_version_from_cargo() {
    // Without a `version` key Tauri uses the crate version.
    assert!(json("tauri.conf.json").get("version").is_none());
}
