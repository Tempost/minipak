fn main() {
    println!("cargo:rustc-link-arg-bin=stage1=-nostartfiles");
    println!("cargo:rustc-link-arg-bin=stage1=-static");
}
