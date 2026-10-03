use crate::game;
use crate::utils::{config_dir, safe_folder_name};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

const DISABLED_SUFFIX: &str = ".disabled";
const LOADER_NAME: &str = "BlueprintLoader";
const PAK_EXT: [&str; 3] = ["pak", "ucas", "utoc"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub paks_path: String,
    pub theme: String,
    pub mods_sort: String,
    pub mods_sort_order: String,
    pub mods_status_filter: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            paks_path: String::new(),
            theme: "dark".to_string(),
            mods_sort: "name".to_string(),
            mods_sort_order: "asc".to_string(),
            mods_status_filter: "all".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModEntry {
    pub folder_name: String,
    pub enabled: bool,
    pub is_loader: bool,
    pub size: u64,
    pub file_count: usize,
    pub created_at: i64,
}

fn config_file() -> PathBuf {
    config_dir().join("config.json")
}

fn read_config() -> Config {
    fs::read_to_string(config_file())
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

fn write_config(cfg: &Config) -> Result<(), String> {
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(config_file(), json).map_err(|e| e.to_string())
}

pub fn paks_dir() -> Result<PathBuf, String> {
    let cfg = read_config();
    let p = PathBuf::from(&cfg.paks_path);
    if cfg.paks_path.is_empty() || !p.is_dir() {
        return Err("Game folder not set. Pick it in Settings.".to_string());
    }
    Ok(p)
}

fn mods_dir() -> Result<PathBuf, String> {
    let d = paks_dir()?.join("~mods");
    fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

fn mod_path(folder_name: &str) -> Result<PathBuf, String> {
    let mut comps = Path::new(folder_name).components();
    match (comps.next(), comps.next()) {
        (Some(Component::Normal(_)), None) => Ok(mods_dir()?.join(folder_name)),
        _ => Err(format!("Invalid mod name: {}", folder_name)),
    }
}

fn is_loader(folder_name: &str) -> bool {
    folder_name.eq_ignore_ascii_case(LOADER_NAME)
}

fn files_of(folder: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(folder)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .collect()
}

fn is_disabled_file(p: &Path) -> bool {
    p.to_string_lossy().ends_with(DISABLED_SUFFIX)
}

fn set_enabled(folder: &Path, enable: bool) -> std::io::Result<()> {
    for f in files_of(folder) {
        let name = f
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if enable && is_disabled_file(&f) {
            fs::rename(
                &f,
                f.with_file_name(&name[..name.len() - DISABLED_SUFFIX.len()]),
            )?;
        } else if !enable && !is_disabled_file(&f) {
            fs::rename(&f, f.with_file_name(format!("{}{}", name, DISABLED_SUFFIX)))?;
        }
    }
    Ok(())
}

fn created_at(path: &Path) -> i64 {
    fs::metadata(path)
        .and_then(|m| m.created().or_else(|_| m.modified()))
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn has_pak_ext(p: &Path) -> bool {
    let name = p.to_string_lossy().to_lowercase();
    let name = name.trim_end_matches(DISABLED_SUFFIX);
    PAK_EXT
        .iter()
        .any(|ext| name.ends_with(&format!(".{}", ext)))
}

#[tauri::command]
pub fn load_config() -> Config {
    read_config()
}

#[tauri::command]
pub fn save_config(config: Config) -> Result<(), String> {
    write_config(&config)
}

#[tauri::command]
pub fn detect_game() -> Vec<String> {
    game::detect_installs()
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect()
}

#[tauri::command]
pub fn set_game_path(path: String) -> Result<Config, String> {
    let paks = game::resolve_paks(Path::new(&path)).ok_or(
        "Couldn't find Content\\Paks in that folder. Pick the Minecraft Dungeons II install folder.",
    )?;
    let mut cfg = read_config();
    cfg.paks_path = paks.to_string_lossy().to_string();
    write_config(&cfg)?;
    Ok(cfg)
}

#[tauri::command]
pub fn get_mods() -> Result<Vec<ModEntry>, String> {
    let dir = mods_dir()?;
    let mut mods: Vec<ModEntry> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| {
            let path = e.path();
            let folder_name = e.file_name().to_string_lossy().to_string();
            let files = files_of(&path);
            ModEntry {
                enabled: files.is_empty() || files.iter().any(|f| !is_disabled_file(f)),
                is_loader: is_loader(&folder_name),
                size: files
                    .iter()
                    .filter_map(|f| f.metadata().ok())
                    .map(|m| m.len())
                    .sum(),
                file_count: files.len(),
                created_at: created_at(&path),
                folder_name,
            }
        })
        .collect();

    mods.sort_by_key(|m| m.folder_name.to_lowercase());
    Ok(mods)
}

#[tauri::command]
pub fn set_mods_enabled(folder_names: Vec<String>, enabled: bool) -> Result<(), String> {
    let mut errors = vec![];
    for name in folder_names.iter().filter(|n| !is_loader(n)) {
        if let Err(e) =
            mod_path(name).and_then(|p| set_enabled(&p, enabled).map_err(|e| e.to_string()))
        {
            errors.push(format!("{}: {}", name, e));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Couldn't rename some files (is the game still running?)\n{}",
            errors.join("\n")
        ))
    }
}

fn map_archive_paths(entries: Vec<(usize, PathBuf)>, stem: &str) -> Vec<(usize, PathBuf)> {
    let is_mods_dir = |c: &Component| {
        c.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("~mods")
    };
    let packaged_with_mods_dir = entries
        .iter()
        .any(|(_, p)| p.components().any(|c| is_mods_dir(&c)));

    if packaged_with_mods_dir {
        entries
            .into_iter()
            .filter_map(|(i, p)| {
                let after: PathBuf = p
                    .components()
                    .skip_while(|c| !is_mods_dir(c))
                    .skip(1)
                    .collect();
                match after.components().count() {
                    0 => None,
                    1 => Some((i, Path::new(stem).join(after))),
                    _ => Some((i, after)),
                }
            })
            .collect()
    } else {
        let tops: BTreeSet<_> = entries.iter().map(|(_, p)| p.components().next()).collect();
        let wrapper = (tops.len() == 1 && entries.iter().all(|(_, p)| p.components().count() > 1))
            .then(|| {
                entries[0]
                    .1
                    .components()
                    .next()
                    .unwrap()
                    .as_os_str()
                    .to_owned()
            });
        let folder = wrapper
            .as_ref()
            .map(|w| safe_folder_name(&w.to_string_lossy()))
            .unwrap_or_else(|| stem.to_string());
        entries
            .into_iter()
            .map(|(i, p)| {
                let rel: PathBuf = if wrapper.is_some() {
                    p.components().skip(1).collect()
                } else {
                    p
                };
                (i, Path::new(&folder).join(rel))
            })
            .collect()
    }
}

#[tauri::command]
pub fn install_mod(path: String, overwrite: bool) -> Result<Vec<String>, String> {
    let src = Path::new(&path);
    let stem = safe_folder_name(&src.file_stem().unwrap_or_default().to_string_lossy());
    let dir = mods_dir()?;

    if !path.to_lowercase().ends_with(".zip") {
        if !has_pak_ext(src) {
            return Err("Pick a .zip, .pak, .utoc or .ucas file".to_string());
        }
        let dest = dir.join(&stem).join(src.file_name().unwrap_or_default());
        if dest.exists() && !overwrite {
            return Err(format!("EXISTS:{}", stem));
        }
        fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::copy(src, &dest).map_err(|e| e.to_string())?;
        return Ok(vec![stem]);
    }

    let file = fs::File::open(src).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    let mut entries: Vec<(usize, PathBuf)> = vec![];
    for i in 0..archive.len() {
        let f = archive.by_index(i).map_err(|e| e.to_string())?;
        if f.is_file() {
            if let Some(name) = f.enclosed_name() {
                entries.push((i, name));
            }
        }
    }
    if !entries.iter().any(|(_, p)| has_pak_ext(p)) {
        return Err("No .pak/.utoc/.ucas files found in this archive".to_string());
    }

    let mapped = map_archive_paths(entries, &stem);

    let targets: BTreeSet<String> = mapped
        .iter()
        .filter_map(|(_, p)| p.components().next())
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    let existing: Vec<&String> = targets.iter().filter(|t| dir.join(t).exists()).collect();
    if !existing.is_empty() {
        if !overwrite {
            return Err(format!(
                "EXISTS:{}",
                existing
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        for t in existing {
            fs::remove_dir_all(dir.join(t)).map_err(|e| e.to_string())?;
        }
    }

    for (i, rel) in &mapped {
        let out = dir.join(rel);
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut f = archive.by_index(*i).map_err(|e| e.to_string())?;
        let mut outfile = fs::File::create(&out).map_err(|e| e.to_string())?;
        std::io::copy(&mut f, &mut outfile).map_err(|e| e.to_string())?;
    }

    Ok(targets.into_iter().collect())
}

#[tauri::command]
pub fn delete_mod(folder_name: String) -> Result<(), String> {
    if is_loader(&folder_name) {
        return Err("The Blueprint Loader is required and can't be deleted.".to_string());
    }
    let p = mod_path(&folder_name)?;
    if p.exists() {
        fs::remove_dir_all(&p).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn rename_mod(old_name: String, new_name: String) -> Result<(), String> {
    if is_loader(&old_name) {
        return Err("The Blueprint Loader can't be renamed.".to_string());
    }
    let new_name = safe_folder_name(&new_name);
    if new_name.is_empty() {
        return Err("Name can't be empty".to_string());
    }
    let old_path = mod_path(&old_name)?;
    let new_path = mod_path(&new_name)?;
    if new_path.exists() {
        return Err(format!("'{}' already exists", new_name));
    }
    fs::rename(old_path, new_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_mods_folder() -> Result<(), String> {
    open::that(mods_dir()?).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_game_folder() -> Result<(), String> {
    let paks = paks_dir()?;
    open::that(game::game_root(&paks).unwrap_or(paks)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_config_dir() -> Result<(), String> {
    open::that(config_dir()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_themes_dir() -> Result<(), String> {
    open::that(config_dir().join("themes")).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_themes() -> Result<Vec<(String, String)>, String> {
    let entries = fs::read_dir(config_dir().join("themes")).map_err(|e| e.to_string())?;
    Ok(entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().map(|e| e == "css").unwrap_or(false))
        .filter_map(|p| {
            let name = p.file_stem()?.to_str()?.to_string();
            Some((name, p.to_string_lossy().to_string()))
        })
        .collect())
}

#[tauri::command]
pub fn read_theme(path: String) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn launch_game() -> Result<(), String> {
    let paks = paks_dir()?;
    if let Some(id) = game::steam_appid(&paks) {
        return open::that(format!("steam://rungameid/{}", id)).map_err(|e| e.to_string());
    }
    let exe = game::find_launch_exe(&paks).ok_or("Couldn't find the game executable")?;
    std::process::Command::new(&exe)
        .current_dir(exe.parent().unwrap_or(Path::new(".")))
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_files_with_disabled_suffix() {
        let dir = std::env::temp_dir().join(format!("mcdii_toggle_test_{}", std::process::id()));
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("Mod.pak"), b"").unwrap();
        fs::write(dir.join("sub").join("Mod.utoc"), b"").unwrap();

        set_enabled(&dir, false).unwrap();
        assert!(dir.join("Mod.pak.disabled").exists());
        assert!(dir.join("sub").join("Mod.utoc.disabled").exists());
        assert!(files_of(&dir).iter().all(|f| is_disabled_file(f)));

        set_enabled(&dir, true).unwrap();
        assert!(dir.join("Mod.pak").exists());
        assert!(dir.join("sub").join("Mod.utoc").exists());

        fs::remove_dir_all(&dir).ok();
    }

    fn mapped(paths: &[&str]) -> Vec<String> {
        let entries = paths
            .iter()
            .enumerate()
            .map(|(i, p)| (i, PathBuf::from(p)))
            .collect();
        map_archive_paths(entries, "Archive")
            .into_iter()
            .map(|(_, p)| p.to_string_lossy().replace('\\', "/"))
            .collect()
    }

    #[test]
    fn maps_archive_layouts() {
        assert_eq!(
            mapped(&["Foo.pak", "Foo.utoc"]),
            ["Archive/Foo.pak", "Archive/Foo.utoc"]
        );
        assert_eq!(
            mapped(&["CoolMod/Foo.pak", "CoolMod/sub/a.txt"]),
            ["CoolMod/Foo.pak", "CoolMod/sub/a.txt"]
        );
        assert_eq!(
            mapped(&[
                "Dungeons/Content/Paks/~mods/A/a.pak",
                "Dungeons/Content/Paks/~mods/B/b.pak",
                "README.txt"
            ]),
            ["A/a.pak", "B/b.pak"]
        );
        assert_eq!(mapped(&["~mods/x.pak"]), ["Archive/x.pak"]);
    }

    #[test]
    fn recognises_pak_files_even_when_disabled() {
        assert!(has_pak_ext(Path::new("a/Foo.PAK")));
        assert!(has_pak_ext(Path::new("Foo.utoc.disabled")));
        assert!(!has_pak_ext(Path::new("readme.txt")));
    }
}
