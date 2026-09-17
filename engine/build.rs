fn main() {
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    // Libraries that not every PC has are loaded on first use, so the engine starts everywhere.
    // The prebuilt ONNX Runtime imports DirectML and Direct3D even though Vorto only runs it on
    // the processor, and DirectML is missing before Windows 10 1903. whisper.cpp's Vulkan backend
    // imports the Vulkan loader, which only graphics drivers install.
    let mut dlls = Vec::new();
    if std::env::var_os("CARGO_FEATURE_PARAKEET").is_some() {
        dlls.extend(["DirectML.dll", "d3d12.dll", "dxgi.dll"]);
    }
    if std::env::var_os("CARGO_FEATURE_VULKAN").is_some() {
        dlls.push("vulkan-1.dll");
    }
    if dlls.is_empty() {
        return;
    }
    for dll in dlls {
        println!("cargo:rustc-link-arg-bins=/DELAYLOAD:{dll}");
    }
    println!("cargo:rustc-link-arg-bins=delayimp.lib");
    // LNK4199: a delay-loaded DLL the binary ends up not importing.
    println!("cargo:rustc-link-arg-bins=/IGNORE:4199");
}
