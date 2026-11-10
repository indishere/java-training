// The game binary has no C runtime: stop the default startfiles and default
// libraries from being linked, per toolchain flavour. Bins only, and `cargo
// test` builds the lib with std and links it normally.
fn main() {
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if env == "msvc" {
        println!("cargo:rustc-link-arg-bins=/NODEFAULTLIB");
        println!("cargo:rustc-link-arg-bins=/DEBUG:NONE"); // no PDB path baked into the exe
    } else if env == "gnu" {
        println!("cargo:rustc-link-arg-bins=-nostartfiles");
        println!("cargo:rustc-link-arg-bins=-nostdlib");
        println!("cargo:rustc-link-arg-bins=-Wl,-e,mainCRTStartup");
    }
}
