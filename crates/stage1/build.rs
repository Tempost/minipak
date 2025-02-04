fn main() {
    for &arg in &["-nostartfiles", "-nodefaultlibs", "-static"] {
        println!("cargo:rustc-link-arg-bin=stage1={}", arg);
    }
}
