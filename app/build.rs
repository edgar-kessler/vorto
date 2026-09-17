fn main() {
    // The web UI is embedded at compile time; rebuild whenever it changes.
    println!("cargo:rerun-if-changed=../ui/dist");
    tauri_build::build()
}
