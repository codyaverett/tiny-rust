fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let linker_script = format!("{}/linker.ld", manifest_dir);
    println!("cargo:rustc-link-arg=-nostartfiles");
    println!("cargo:rustc-link-arg=-nodefaultlibs");
    println!("cargo:rustc-link-arg=-static");
    println!("cargo:rustc-link-arg=-T{}", linker_script);
    println!("cargo:rerun-if-changed=linker.ld");
}
