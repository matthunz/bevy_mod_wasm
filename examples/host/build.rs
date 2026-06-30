use std::{env, path::PathBuf, process::Command};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let guest_dir = manifest_dir.parent().unwrap().join("guest");

    println!(
        "cargo:rerun-if-changed={}",
        guest_dir.join("src/lib.rs").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        guest_dir.join("Cargo.toml").display()
    );

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let status = Command::new(&cargo)
        .args([
            "build",
            "--manifest-path",
            guest_dir.join("Cargo.toml").to_str().unwrap(),
            "--target",
            "wasm32-unknown-unknown",
            "--release",
        ])
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .status()
        .expect("failed to spawn guest build");

    assert!(status.success(), "guest wasm build failed");
    let target_dir = PathBuf::from(env::var("OUT_DIR").unwrap())
        .ancestors()
        .find(|p| p.file_name().is_some_and(|n| n == "target"))
        .expect("OUT_DIR should be under a target dir")
        .to_path_buf();

    let wasm_path =
        target_dir.join("wasm32-unknown-unknown/release/bevy_mod_wasm_example_guest.wasm");
    println!("cargo:rustc-env=GUEST_WASM_PATH={}", wasm_path.display());
}
