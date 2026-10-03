fn main() {
    println!("cargo:rerun-if-changed=../package.json");

    let pkg = std::fs::read_to_string("../package.json").expect("Failed to read package.json");
    let json: serde_json::Value = serde_json::from_str(&pkg).expect("Failed to parse package.json");
    let version = json["version"]
        .as_str()
        .expect("package.json missing 'version' field");
    println!("cargo:rustc-env=APP_VERSION={}", version);

    tauri_build::build()
}
