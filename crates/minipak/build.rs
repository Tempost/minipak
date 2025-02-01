fn main() {
    println!("cargo:rustc-link-arg-bin=minipak=-nostartfiles");
    println!("cargo:rustc-link-arg-bin=minipak=-static");
}
