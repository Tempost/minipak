fn main() {
    println!("cargo:rustc-link-arg-bin=minipak=-nostartfiles");
    println!("cargo:rustc-link-arg-bin=minipak=-static");
    println!("cargo:rustc-link-arg-bin=minipak=-nodefaultlibs");
}
