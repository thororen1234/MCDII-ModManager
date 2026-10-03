use crate::mod_manager::paks_dir;
use crate::utils::config_dir;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

const NO_INTRO_FILES: [(&str, &[u8]); 4] = [
    (
        "blank_splash720.mp4",
        include_bytes!("../../noIntro/blank_splash720.mp4"),
    ),
    (
        "loader_splash1080WithAudio.mp4",
        include_bytes!("../../noIntro/loader_splash1080WithAudio.mp4"),
    ),
    (
        "splashscreen-long-1080.mp4",
        include_bytes!("../../noIntro/splashscreen-long-1080.mp4"),
    ),
    (
        "splashscreen-long-720.mp4",
        include_bytes!("../../noIntro/splashscreen-long-720.mp4"),
    ),
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntroStatus {
    pub applied: bool,
    pub can_restore: bool,
}

fn movies_dir() -> Result<PathBuf, String> {
    let dir = paks_dir()?
        .parent()
        .ok_or("No Content folder")?
        .join("Movies");
    if !dir.is_dir() {
        return Err(format!("Movies folder not found: {}", dir.display()));
    }
    Ok(dir)
}

fn backup_dir() -> PathBuf {
    config_dir().join("intro-backup")
}

fn is_blank(path: &PathBuf, blank: &[u8]) -> bool {
    fs::read(path).map(|b| b == blank).unwrap_or(false)
}

#[tauri::command]
pub fn intro_status() -> Result<IntroStatus, String> {
    let movies = movies_dir()?;
    Ok(IntroStatus {
        applied: NO_INTRO_FILES
            .iter()
            .all(|(name, blank)| is_blank(&movies.join(name), blank)),
        can_restore: NO_INTRO_FILES
            .iter()
            .any(|(name, _)| backup_dir().join(name).exists()),
    })
}

#[tauri::command]
pub fn apply_no_intro() -> Result<(), String> {
    let movies = movies_dir()?;
    let backup = backup_dir();
    fs::create_dir_all(&backup).map_err(|e| e.to_string())?;

    for (name, blank) in NO_INTRO_FILES {
        let target = movies.join(name);
        if target.exists() && !is_blank(&target, blank) {
            fs::copy(&target, backup.join(name))
                .map_err(|e| format!("Backing up {}: {}", name, e))?;
        }
        fs::write(&target, blank)
            .map_err(|e| format!("Writing {}: {} (is the game still running?)", name, e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn restore_intro() -> Result<(), String> {
    let movies = movies_dir()?;
    let backup = backup_dir();
    let mut restored = 0;
    for (name, _) in NO_INTRO_FILES {
        let saved = backup.join(name);
        if saved.exists() {
            fs::copy(&saved, movies.join(name))
                .map_err(|e| format!("Restoring {}: {} (is the game still running?)", name, e))?;
            restored += 1;
        }
    }
    if restored == 0 {
        return Err(
            "No backup of the original intro movies. Repair the game files to get them back."
                .to_string(),
        );
    }
    Ok(())
}
