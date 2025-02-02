use std::process::Command;

fn main() {
    cargo_build("../stage1");
    println!("cargo:rustc-link-arg-bin=minipak=-nostartfiles");
    println!("cargo:rustc-link-arg-bin=minipak=-static");
}

fn cargo_build(path: &str) {
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
            "Building {} failed.\nStdout: {}\nStderr: {}",
            path,
            String::from_utf8_lossy(&output.stdout[..]),
            String::from_utf8_lossy(&output.stderr[..])
        );
    }
}
