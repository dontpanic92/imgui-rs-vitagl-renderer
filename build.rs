fn main() {
    cc::Build::new()
        .file("src/imgui_vita_touch_wrapper.cpp")
        .file("imgui-vita/imgui_vita_touch.cpp")
        .compile("imgui_vita_touch");

    if let Ok(sdk) = std::env::var("VITASDK").map(std::path::PathBuf::from) {
        let lib_dir = sdk.join("arm-vita-eabi").join("lib");
        println!("cargo:rustc-link-search={}", lib_dir.to_str().unwrap());
    } else {
        println!("cargo:warning=$VITASDK not set!");
    }
}
