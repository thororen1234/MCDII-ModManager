use std::fs;
use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    let base = dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));
    let path = base.join("MCDII-ModManager");
    let _ = fs::create_dir_all(path.join("themes"));
    path
}

pub fn safe_folder_name(text: &str) -> String {
    let invalid = r#"\/:*?"<>|"#;
    text.chars()
        .map(|c| if invalid.contains(c) { '_' } else { c })
        .collect::<String>()
        .trim()
        .trim_matches('.')
        .to_string()
}
