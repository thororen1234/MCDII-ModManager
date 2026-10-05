use crate::game;
use crate::utils::{config_dir, safe_folder_name};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

const DISABLED_SUFFIX: &str = ".disabled";
const PAK_EXT: [&str; 3] = ["pak", "ucas", "utoc"];

/// Loaders the game can use to run mods. Only one can work at a time: both change the same game file.
const BUNDLED_LOADER: &str = "BetterBlueprintLoader";
const LOADER_NAMES: [&str; 2] = [BUNDLED_LOADER, "BlueprintLoader"];

/// Releases of the mods repo, where BetterBlueprintLoader updates come from.
const LOADER_RELEASES_URL: &str =
    "https://api.github.com/repos/thororen1234/MCDII-Mods/releases?per_page=100";

/// Index of the .ucas in BUNDLED_LOADER_FILES, which holds the loader's version text.
const LOADER_UCAS: usize = 1;
const BUNDLED_LOADER_FILES: [(&str, &[u8]); 3] = [
    (
        "BetterBlueprintLoader_P.pak",
        include_bytes!("../../betterBlueprintLoader/BetterBlueprintLoader_P.pak"),
    ),
    (
        "BetterBlueprintLoader_P.ucas",
        include_bytes!("../../betterBlueprintLoader/BetterBlueprintLoader_P.ucas"),
    ),
    (
        "BetterBlueprintLoader_P.utoc",
        include_bytes!("../../betterBlueprintLoader/BetterBlueprintLoader_P.utoc"),
    ),
];

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
    /// The bundled BetterBlueprintLoader is newer than this installed one.
    pub update_available: bool,
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
    ensure_loader(&d)?;
    Ok(d)
}

/// Installs BetterBlueprintLoader unless a loader is already there, and puts back any of its
/// files that went missing. A player who uses Blueprint Loader instead keeps it.
fn ensure_loader(mods: &Path) -> Result<(), String> {
    let installed = fs::read_dir(mods)
        .map_err(|e| e.to_string())?
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|name| is_loader(name))
        .collect::<Vec<_>>();
    let folder = match installed
        .iter()
        .find(|n| n.eq_ignore_ascii_case(BUNDLED_LOADER))
    {
        Some(name) => mods.join(name),
        None if installed.is_empty() => mods.join(BUNDLED_LOADER),
        None => return Ok(()),
    };
    write_loader_files(&folder, &BUNDLED_LOADER_FILES, false)
}

/// Writes BetterBlueprintLoader files into its folder: all of them, or only missing ones.
fn write_loader_files(folder: &Path, files: &[(&str, &[u8])], replace: bool) -> Result<(), String> {
    fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    for (name, bytes) in files {
        let target = folder.join(name);
        let disabled = folder.join(format!("{}{}", name, DISABLED_SUFFIX));
        if replace || (!target.exists() && !disabled.exists()) {
            let _ = fs::remove_file(&disabled);
            fs::write(&target, bytes).map_err(|e| {
                format!(
                    "Installing {} ({}): {} (is the game still running?)",
                    BUNDLED_LOADER, name, e
                )
            })?;
        }
    }
    Ok(())
}

/// The version BetterBlueprintLoader shows in game ("BetterBlueprintLoader 1.0.1"), read from its .ucas.
fn loader_version(ucas: &[u8]) -> Option<Vec<u32>> {
    let needle = format!("{} ", BUNDLED_LOADER).into_bytes();
    ucas.windows(needle.len())
        .enumerate()
        .filter(|(_, w)| *w == needle.as_slice())
        .find_map(|(i, _)| {
            let rest = &ucas[i + needle.len()..];
            let end = rest
                .iter()
                .position(|b| !(b.is_ascii_digit() || *b == b'.'))
                .unwrap_or(rest.len());
            parse_version(std::str::from_utf8(&rest[..end]).ok()?)
        })
}

fn parse_version(text: &str) -> Option<Vec<u32>> {
    let parts: Option<Vec<u32>> = text
        .trim_end_matches('.')
        .split('.')
        .map(|p| p.parse().ok())
        .collect();
    parts.filter(|p| p.len() >= 2)
}

fn version_string(version: &[u32]) -> String {
    version
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(".")
}

/// The newest BetterBlueprintLoader release on GitHub, found when the app starts.
#[derive(Debug, Clone)]
struct RemoteLoader {
    version: Vec<u32>,
    zip_url: String,
}

static LATEST_LOADER: Mutex<Option<RemoteLoader>> = Mutex::new(None);

fn latest_loader() -> Option<RemoteLoader> {
    LATEST_LOADER.lock().ok()?.clone()
}

/// Whether a newer BetterBlueprintLoader than the one in this folder is available, from GitHub or
/// bundled with the app. A newer install (from a newer download) isn't offered an older one.
fn loader_update_available(folder: &Path) -> bool {
    let installed: Vec<Option<Vec<u8>>> = BUNDLED_LOADER_FILES
        .iter()
        .map(|(name, _)| fs::read(folder.join(name)).ok())
        .collect();
    let newest = [
        loader_version(BUNDLED_LOADER_FILES[LOADER_UCAS].1),
        latest_loader().map(|l| l.version),
    ]
    .into_iter()
    .flatten()
    .max();
    match (
        installed[LOADER_UCAS].as_deref().and_then(loader_version),
        newest,
    ) {
        (Some(have), Some(newest)) => have < newest,
        // The installed version can't be read: offer the bundled one if the files differ.
        _ => installed
            .iter()
            .zip(BUNDLED_LOADER_FILES.iter())
            .any(|(file, (_, bytes))| file.as_deref() != Some(*bytes)),
    }
}

fn http_client() -> Result<reqwest::Client, String> {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    reqwest::Client::builder()
        .user_agent("MCDII-ModManager")
        .build()
        .map_err(|e| e.to_string())
}

/// Finds the newest BetterBlueprintLoader release in the mods repo. The repo releases several mods,
/// so GitHub's "latest release" can be another mod's: pick the newest BetterBlueprintLoader-v* tag.
async fn fetch_latest_loader() -> Result<Option<RemoteLoader>, String> {
    let releases: serde_json::Value = http_client()?
        .get(LOADER_RELEASES_URL)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(newest_loader_release(&releases))
}

fn newest_loader_release(releases: &serde_json::Value) -> Option<RemoteLoader> {
    let tag_prefix = format!("{}-v", BUNDLED_LOADER);
    releases
        .as_array()?
        .iter()
        .filter(|r| {
            !r["draft"].as_bool().unwrap_or(true) && !r["prerelease"].as_bool().unwrap_or(false)
        })
        .filter_map(|r| {
            let version = parse_version(r["tag_name"].as_str()?.strip_prefix(&tag_prefix)?)?;
            let zip_url = r["assets"].as_array()?.iter().find(|a| {
                a["name"]
                    .as_str()
                    .is_some_and(|n| n.to_lowercase().ends_with(".zip"))
            })?["browser_download_url"]
                .as_str()?
                .to_string();
            Some(RemoteLoader { version, zip_url })
        })
        .max_by(|a, b| a.version.cmp(&b.version))
}

/// Checks GitHub for a newer BetterBlueprintLoader. Returns its version when it's newer than the
/// installed one, so the app can say so.
#[tauri::command]
pub async fn check_loader_update() -> Result<Option<String>, String> {
    let latest = fetch_latest_loader().await?;
    if let Ok(mut cached) = LATEST_LOADER.lock() {
        cached.clone_from(&latest);
    }
    let folder = mods_dir()?.join(BUNDLED_LOADER);
    Ok(latest
        .filter(|_| folder.is_dir() && loader_update_available(&folder))
        .map(|l| version_string(&l.version)))
}

/// The loader's files from a release zip, wherever they are inside it.
fn loader_files_from_zip(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let wanted: Vec<&str> = BUNDLED_LOADER_FILES
        .iter()
        .map(|(n, _)| *n)
        .chain(["READ_THIS_FILE.txt"])
        .collect();
    let mut files = vec![];
    for i in 0..archive.len() {
        let mut f = archive.by_index(i).map_err(|e| e.to_string())?;
        // Zips made by Windows PowerShell use backslashes.
        let name = f.name().replace('\\', "/");
        let base = name.rsplit('/').next().unwrap_or_default().to_string();
        if f.is_file() && wanted.iter().any(|w| w.eq_ignore_ascii_case(&base)) {
            let mut data = vec![];
            std::io::Read::read_to_end(&mut f, &mut data).map_err(|e| e.to_string())?;
            files.push((base, data));
        }
    }
    for (name, _) in BUNDLED_LOADER_FILES {
        if !files.iter().any(|(n, _)| n.eq_ignore_ascii_case(name)) {
            return Err(format!(
                "The {} download is missing {}",
                BUNDLED_LOADER, name
            ));
        }
    }
    Ok(files)
}

/// Updates BetterBlueprintLoader to the newest version: downloaded from GitHub, or the bundled one
/// if that's newer. Returns the version installed.
#[tauri::command]
pub async fn update_loader() -> Result<String, String> {
    let folder = mod_path(BUNDLED_LOADER)?;
    let bundled = loader_version(BUNDLED_LOADER_FILES[LOADER_UCAS].1);
    match latest_loader().filter(|l| Some(&l.version) > bundled.as_ref()) {
        Some(latest) => {
            let bytes = http_client()?
                .get(&latest.zip_url)
                .send()
                .await
                .and_then(|r| r.error_for_status())
                .map_err(|e| format!("Downloading {}: {}", BUNDLED_LOADER, e))?
                .bytes()
                .await
                .map_err(|e| e.to_string())?;
            let files = loader_files_from_zip(&bytes)?;
            let refs: Vec<(&str, &[u8])> = files
                .iter()
                .map(|(n, d)| (n.as_str(), d.as_slice()))
                .collect();
            write_loader_files(&folder, &refs, true)?;
            Ok(version_string(&latest.version))
        }
        None => {
            write_loader_files(&folder, &BUNDLED_LOADER_FILES, true)?;
            Ok(bundled.map(|v| version_string(&v)).unwrap_or_default())
        }
    }
}

fn mod_path(folder_name: &str) -> Result<PathBuf, String> {
    let mut comps = Path::new(folder_name).components();
    match (comps.next(), comps.next()) {
        (Some(Component::Normal(_)), None) => Ok(mods_dir()?.join(folder_name)),
        _ => Err(format!("Invalid mod name: {}", folder_name)),
    }
}

fn is_loader(folder_name: &str) -> bool {
    LOADER_NAMES
        .iter()
        .any(|n| folder_name.eq_ignore_ascii_case(n))
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
                enabled: is_enabled(&files),
                is_loader: is_loader(&folder_name),
                size: files
                    .iter()
                    .filter_map(|f| f.metadata().ok())
                    .map(|m| m.len())
                    .sum(),
                file_count: files.len(),
                created_at: created_at(&path),
                update_available: folder_name.eq_ignore_ascii_case(BUNDLED_LOADER)
                    && loader_update_available(&path),
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

fn strip_nexus_suffix(stem: &str) -> &str {
    // [key, date, version, mod id, name]
    let parts: Vec<&str> = stem.rsplitn(5, ' ').collect();
    let is_date = |s: &str| {
        s.len() == 17
            && s.char_indices().all(|(i, c)| match i {
                4 | 7 | 13 => c == '-',
                10 => c == 'T',
                16 => c == 'Z',
                _ => c.is_ascii_digit(),
            })
    };
    match parts[..] {
        [key, date, version, id, name]
            if key.chars().all(|c| c.is_ascii_alphanumeric())
                && is_date(date)
                && !version.is_empty()
                && id.chars().all(|c| c.is_ascii_digit())
                && !name.trim().is_empty() =>
        {
            name.trim()
        }
        _ => stem,
    }
}

fn pak_key(p: &Path) -> Option<String> {
    let name = p.file_name()?.to_string_lossy().to_lowercase();
    has_pak_ext(p).then(|| name.trim_end_matches(DISABLED_SUFFIX).to_string())
}

fn find_installed(dir: &Path, keys: &BTreeSet<String>) -> Option<String> {
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .find(|e| {
            files_of(&e.path())
                .iter()
                .any(|f| pak_key(f).is_some_and(|k| keys.contains(&k)))
        })
        .map(|e| e.file_name().to_string_lossy().to_string())
}

fn is_enabled(files: &[PathBuf]) -> bool {
    files.is_empty() || files.iter().any(|f| !is_disabled_file(f))
}

fn prepare_update(folder: &Path) -> Result<bool, String> {
    let was_disabled = !is_enabled(&files_of(folder));
    set_enabled(folder, true).map_err(|e| e.to_string())?;
    for f in files_of(folder).iter().filter(|f| has_pak_ext(f)) {
        fs::remove_file(f).map_err(|e| e.to_string())?;
    }
    Ok(was_disabled)
}

#[tauri::command]
pub fn install_mod(path: String, overwrite: bool) -> Result<Vec<String>, String> {
    install_into(&mods_dir()?, Path::new(&path), overwrite)
}

fn install_into(dir: &Path, src: &Path, overwrite: bool) -> Result<Vec<String>, String> {
    let stem = safe_folder_name(strip_nexus_suffix(
        &src.file_stem().unwrap_or_default().to_string_lossy(),
    ));

    if !src.to_string_lossy().to_lowercase().ends_with(".zip") {
        let key = pak_key(src).ok_or("Pick a .zip, .pak, .utoc or .ucas file")?;
        let folder = find_installed(dir, &BTreeSet::from([key])).unwrap_or(stem);
        let file_name = src.file_name().unwrap_or_default();
        let dest = dir.join(&folder).join(file_name);
        let disabled_dest = dir.join(&folder).join(format!(
            "{}{}",
            file_name.to_string_lossy(),
            DISABLED_SUFFIX
        ));
        if (dest.exists() || disabled_dest.exists()) && !overwrite {
            return Err(format!("EXISTS:{}", folder));
        }
        let was_disabled = dir.join(&folder).exists() && !is_enabled(&files_of(&dir.join(&folder)));
        fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
        set_enabled(&dir.join(&folder), true).map_err(|e| e.to_string())?;
        fs::copy(src, &dest).map_err(|e| e.to_string())?;
        if was_disabled {
            set_enabled(&dir.join(&folder), false).map_err(|e| e.to_string())?;
        }
        return Ok(vec![folder]);
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

    let mapped = point_at_installed(dir, map_archive_paths(entries, &stem));

    let targets: BTreeSet<String> = mapped
        .iter()
        .filter_map(|(_, p)| p.components().next())
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    let existing: Vec<&String> = targets.iter().filter(|t| dir.join(t).exists()).collect();
    let mut keep_disabled = vec![];
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
            if prepare_update(&dir.join(t))? {
                keep_disabled.push(dir.join(t));
            }
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
    for folder in keep_disabled {
        set_enabled(&folder, false).map_err(|e| e.to_string())?;
    }

    Ok(targets.into_iter().collect())
}

fn point_at_installed(dir: &Path, mapped: Vec<(usize, PathBuf)>) -> Vec<(usize, PathBuf)> {
    let top = |p: &Path| {
        p.components()
            .next()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
    };
    let mut keys: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (_, p) in &mapped {
        if let (Some(t), Some(k)) = (top(p), pak_key(p)) {
            keys.entry(t).or_default().insert(k);
        }
    }
    let installed: BTreeMap<String, String> = keys
        .iter()
        .filter_map(|(t, k)| find_installed(dir, k).map(|f| (t.clone(), f)))
        .collect();

    mapped
        .into_iter()
        .map(|(i, p)| match top(&p).and_then(|t| installed.get(&t)) {
            Some(folder) => (
                i,
                Path::new(folder).join(p.components().skip(1).collect::<PathBuf>()),
            ),
            None => (i, p),
        })
        .collect()
}

#[tauri::command]
pub fn delete_mod(folder_name: String) -> Result<(), String> {
    // Deleting Blueprint Loader is how a player switches to BetterBlueprintLoader, which is then
    // installed in its place.
    if folder_name.eq_ignore_ascii_case(BUNDLED_LOADER) {
        return Err(format!(
            "{} runs your mods and can't be deleted.",
            BUNDLED_LOADER
        ));
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
        return Err("Mod loaders can't be renamed.".to_string());
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
    fn installs_bundled_loader_only_when_no_loader() {
        let dir = std::env::temp_dir().join(format!("mcdii_loader_test_{}", std::process::id()));

        // No loader: BetterBlueprintLoader is installed.
        fs::create_dir_all(&dir).unwrap();
        ensure_loader(&dir).unwrap();
        for (name, bytes) in &BUNDLED_LOADER_FILES {
            assert_eq!(
                &fs::read(dir.join(BUNDLED_LOADER).join(name)).unwrap(),
                bytes
            );
        }

        // Missing files are put back, existing ones are left alone.
        let loader = dir.join(BUNDLED_LOADER);
        fs::write(loader.join(BUNDLED_LOADER_FILES[0].0), b"custom").unwrap();
        fs::remove_file(loader.join(BUNDLED_LOADER_FILES[1].0)).unwrap();
        ensure_loader(&dir).unwrap();
        assert_eq!(
            fs::read(loader.join(BUNDLED_LOADER_FILES[0].0)).unwrap(),
            b"custom"
        );
        assert!(loader.join(BUNDLED_LOADER_FILES[1].0).exists());

        // Blueprint Loader already there: nothing is added next to it.
        fs::remove_dir_all(&loader).unwrap();
        fs::create_dir_all(dir.join("blueprintloader")).unwrap();
        ensure_loader(&dir).unwrap();
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn offers_loader_update_only_when_bundled_is_newer() {
        let bundled = loader_version(BUNDLED_LOADER_FILES[LOADER_UCAS].1).expect("bundled version");
        assert_eq!(
            loader_version(b"\0BetterBlueprintLoader \0x\0BetterBlueprintLoader 1.0.1\0"),
            Some(vec![1, 0, 1])
        );

        let dir =
            std::env::temp_dir().join(format!("mcdii_loader_update_test_{}", std::process::id()));
        write_loader_files(&dir, &BUNDLED_LOADER_FILES, true).unwrap();
        assert!(!loader_update_available(&dir), "same files");

        let ucas = dir.join(BUNDLED_LOADER_FILES[LOADER_UCAS].0);
        fs::write(&ucas, b"\0BetterBlueprintLoader 0.0.1\0").unwrap();
        assert!(loader_update_available(&dir), "older install");

        let newer = format!("\0BetterBlueprintLoader {}.0.0\0", bundled[0] + 1);
        fs::write(&ucas, newer).unwrap();
        assert!(
            !loader_update_available(&dir),
            "newer install isn't downgraded"
        );

        write_loader_files(&dir, &BUNDLED_LOADER_FILES, true).unwrap();
        assert!(!loader_update_available(&dir), "updated");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn picks_newest_loader_release() {
        let release = |tag: &str, draft: bool| {
            serde_json::json!({
                "tag_name": tag, "draft": draft, "prerelease": false,
                "assets": [{ "name": format!("{}.zip", tag), "browser_download_url": format!("https://x/{}.zip", tag) }]
            })
        };
        let releases = serde_json::json!([
            release("Emoticons-v9.0.0", false),
            release("BetterBlueprintLoader-v1.0.1", false),
            release("BetterBlueprintLoader-v1.0.10", false),
            release("BetterBlueprintLoader-v2.0.0", true),
            release("BetterBlueprintLoader-v1.0.2", false),
        ]);
        let newest = newest_loader_release(&releases).unwrap();
        assert_eq!(newest.version, [1, 0, 10]);
        assert_eq!(
            newest.zip_url,
            "https://x/BetterBlueprintLoader-v1.0.10.zip"
        );
        assert!(
            newest_loader_release(&serde_json::json!([release("Emoticons-v1.0.0", false)]))
                .is_none()
        );
    }

    #[test]
    fn reads_loader_files_from_release_zip() {
        let path =
            std::env::temp_dir().join(format!("mcdii_loader_zip_{}.zip", std::process::id()));
        let mut entries: Vec<(String, &[u8])> = BUNDLED_LOADER_FILES
            .iter()
            .map(|(n, b)| (format!("BetterBlueprintLoader\\{}", n), *b))
            .collect();
        entries.push((
            "BetterBlueprintLoader\\READ_THIS_FILE.txt".into(),
            b"readme",
        ));
        let refs: Vec<(&str, &[u8])> = entries.iter().map(|(n, b)| (n.as_str(), *b)).collect();
        write_zip(&path, &refs);

        let files = loader_files_from_zip(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(files.len(), 4);
        assert!(files
            .iter()
            .any(|(n, d)| n == BUNDLED_LOADER_FILES[0].0 && d == BUNDLED_LOADER_FILES[0].1));

        write_zip(
            &path,
            &[("BetterBlueprintLoader/READ_THIS_FILE.txt", b"readme")],
        );
        assert!(loader_files_from_zip(&fs::read(&path).unwrap()).is_err());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn strips_nexus_download_suffix() {
        assert_eq!(
            strip_nexus_suffix("CustomSkinLoader 9 1.1 2026-09-30T20-56Z ZyayfKaWn"),
            "CustomSkinLoader"
        );
        assert_eq!(
            strip_nexus_suffix("Just Keep Rollin 12 1.0.2 2026-10-04T01-02Z a1B2"),
            "Just Keep Rollin"
        );
        assert_eq!(strip_nexus_suffix("CustomSkinLoader"), "CustomSkinLoader");
        assert_eq!(strip_nexus_suffix("My Mod v2 final"), "My Mod v2 final");
    }

    fn write_zip(path: &Path, files: &[(&str, &[u8])]) {
        use std::io::Write;
        let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
        for (name, data) in files {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn updates_installed_mod_in_place() {
        let root = std::env::temp_dir().join(format!("mcdii_update_test_{}", std::process::id()));
        let mods = root.join("~mods");
        let installed = mods.join("CustomSkinLoader");
        fs::create_dir_all(installed.join("pak")).unwrap();
        fs::create_dir_all(installed.join("skins")).unwrap();
        fs::write(installed.join("pak").join("CustomSkinLoader_P.pak"), b"old").unwrap();
        fs::write(installed.join("pak").join("Removed_P.pak"), b"old").unwrap();
        fs::write(installed.join("skins").join("Mine.png"), b"skin").unwrap();
        set_enabled(&installed, false).unwrap();

        let zip = root.join("CustomSkinLoader 9 1.1 2026-09-30T20-56Z ZyayfKaWn.zip");
        write_zip(
            &zip,
            &[
                ("pak/CustomSkinLoader_P.pak", b"new"),
                ("pak/CustomSkinLoader_P.utoc", b"new"),
                ("READ_THIS_FILE.txt", b"readme"),
            ],
        );

        let err = install_into(&mods, &zip, false).unwrap_err();
        assert_eq!(err, "EXISTS:CustomSkinLoader");
        assert_eq!(
            install_into(&mods, &zip, true).unwrap(),
            ["CustomSkinLoader"]
        );

        assert_eq!(fs::read_dir(&mods).unwrap().count(), 1, "no second copy");
        assert_eq!(
            fs::read(
                installed
                    .join("pak")
                    .join("CustomSkinLoader_P.pak.disabled")
            )
            .unwrap(),
            b"new"
        );
        assert!(installed
            .join("pak")
            .join("CustomSkinLoader_P.utoc.disabled")
            .exists());
        assert!(
            !installed
                .join("pak")
                .join("Removed_P.pak.disabled")
                .exists(),
            "stale pak removed"
        );
        assert_eq!(
            fs::read(installed.join("skins").join("Mine.png.disabled")).unwrap(),
            b"skin",
            "user files kept"
        );

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn fresh_install_uses_name_without_nexus_suffix() {
        let root = std::env::temp_dir().join(format!("mcdii_fresh_test_{}", std::process::id()));
        let mods = root.join("~mods");
        fs::create_dir_all(&mods).unwrap();
        let zip = root.join("CustomSkinLoader 9 1.1 2026-09-30T20-56Z ZyayfKaWn.zip");
        write_zip(
            &zip,
            &[
                ("pak/CustomSkinLoader_P.pak", b"new"),
                ("READ_THIS_FILE.txt", b"readme"),
            ],
        );

        assert_eq!(
            install_into(&mods, &zip, false).unwrap(),
            ["CustomSkinLoader"]
        );
        assert!(mods
            .join("CustomSkinLoader")
            .join("pak")
            .join("CustomSkinLoader_P.pak")
            .exists());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn recognises_pak_files_even_when_disabled() {
        assert!(has_pak_ext(Path::new("a/Foo.PAK")));
        assert!(has_pak_ext(Path::new("Foo.utoc.disabled")));
        assert!(!has_pak_ext(Path::new("readme.txt")));
    }
}
