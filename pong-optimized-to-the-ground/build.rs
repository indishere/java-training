// The game binary has no C runtime: stop rustc's default msvcrt.lib from being linked.
// Bins only, so `cargo test` (which uses std) still links normally.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/NODEFAULTLIB");
        println!("cargo:rustc-link-arg-bins=/DEBUG:NONE"); // no PDB path baked into the exe
    }
}
