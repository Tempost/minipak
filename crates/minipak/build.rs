use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    cargo_build(&PathBuf::from("../stage1"));
    for &arg in &["-nostartfiles", "-nodefaultlibs", "-static"] {
        println!("cargo:rustc-link-arg-bin=minipak={}", arg);
    }
}

fn cargo_build(path: &Path) {
    println!("cargo:rerun-if-changed=..");

    let target_dir = format!("{}/embeds", std::env::var("OUT_DIR").unwrap());

    let output = Command::new("cargo")
        .arg("build")
        .arg("--target-dir")
        .arg(target_dir)
        .arg("--release")
        .current_dir(path)
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();

    if !output.status.success() {
        panic!(
            "Building {:?} failed.\nStdout: {}\nStderr: {}",
            path,
            String::from_utf8_lossy(&output.stdout[..]),
            String::from_utf8_lossy(&output.stderr[..])
        );
    }
}
