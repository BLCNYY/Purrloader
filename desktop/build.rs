fn main() {
    println!("cargo:rerun-if-changed=../VERSION");
    println!("cargo:rerun-if-changed=assets/icon.ico");
    let version = std::fs::read_to_string("../VERSION").expect("read VERSION");
    println!("cargo:rustc-env=PURRLOADER_VERSION={}", version.trim());
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon.ico");
        resource.set("ProductName", "Purrloader");
        resource.set("FileDescription", "Purrloader");
        resource.set("ProductVersion", version.trim());
        resource.compile().expect("compile Windows resources");
    }
}
