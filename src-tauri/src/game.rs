use std::fs;
use std::path::{Path, PathBuf};

fn name_lower(p: &Path) -> String {
    p.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase()
}

pub fn is_game_folder(p: &Path) -> bool {
    let n: String = name_lower(p)
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();
    match n.strip_prefix("minecraftdungeons") {
        Some(rest) => rest.starts_with("ii") || rest.starts_with('2'),
        None => false,
    }
}

fn is_paks_dir(p: &Path) -> bool {
    name_lower(p) == "paks"
        && p.parent()
            .map(|c| name_lower(c) == "content")
            .unwrap_or(false)
        && p.is_dir()
}

fn find_paks_below(root: &Path, max_depth: usize) -> Option<PathBuf> {
    walkdir::WalkDir::new(root)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(|e| {
            let n = e.file_name().to_string_lossy().to_lowercase();
            e.file_type().is_dir() && n != "engine" && n != "~mods" && !n.starts_with('.')
        })
        .flatten()
        .map(|e| e.into_path())
        .find(|p| is_paks_dir(p))
}

pub fn resolve_paks(input: &Path) -> Option<PathBuf> {
    let start = if input.is_file() {
        input.parent()?
    } else {
        input
    };

    if let Some(p) = start.ancestors().find(|a| is_paks_dir(a)) {
        return Some(p.to_path_buf());
    }
    if let Some(p) = find_paks_below(start, 5) {
        return Some(p);
    }
    start
        .ancestors()
        .take(6)
        .find(|a| is_game_folder(a))
        .and_then(|root| find_paks_below(root, 5))
}

pub fn game_root(paks: &Path) -> Option<PathBuf> {
    paks.ancestors()
        .find(|a| is_game_folder(a))
        .map(|a| a.to_path_buf())
        .or_else(|| paks.ancestors().nth(3).map(|a| a.to_path_buf()))
}

fn steam_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = vec![];

    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        use winreg::RegKey;
        let keys = [
            (HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath"),
            (
                HKEY_LOCAL_MACHINE,
                r"SOFTWARE\WOW6432Node\Valve\Steam",
                "InstallPath",
            ),
            (HKEY_LOCAL_MACHINE, r"SOFTWARE\Valve\Steam", "InstallPath"),
        ];
        for (hive, path, value) in keys {
            if let Ok(key) = RegKey::predef(hive).open_subkey(path) {
                if let Ok(v) = key.get_value::<String, _>(value) {
                    roots.push(PathBuf::from(v));
                }
            }
        }
        roots.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
        roots.push(PathBuf::from(r"C:\Program Files\Steam"));
    }

    #[cfg(not(windows))]
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".steam/steam"));
        roots.push(home.join(".local/share/Steam"));
        roots.push(home.join("Library/Application Support/Steam"));
    }

    roots
}

fn vdf_value(line: &str, key: &str) -> Option<String> {
    let parts: Vec<&str> = line.trim().split('"').collect();
    if parts.len() >= 4 && parts[1].eq_ignore_ascii_case(key) {
        Some(parts[3].replace("\\\\", "\\"))
    } else {
        None
    }
}

fn steam_libraries() -> Vec<PathBuf> {
    let mut libs: Vec<PathBuf> = vec![];
    for root in steam_roots() {
        if !root.is_dir() {
            continue;
        }
        libs.push(root.clone());
        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(content) = fs::read_to_string(vdf) {
            libs.extend(
                content
                    .lines()
                    .filter_map(|l| vdf_value(l, "path"))
                    .map(PathBuf::from),
            );
        }
    }
    let mut seen = vec![];
    libs.retain(|l| {
        let key = l.to_string_lossy().to_lowercase().replace('/', "\\");
        let key = key.trim_end_matches('\\').to_string();
        if seen.contains(&key) {
            false
        } else {
            seen.push(key);
            true
        }
    });
    libs
}

fn steam_installs() -> Vec<PathBuf> {
    let mut found = vec![];
    for lib in steam_libraries() {
        let Ok(entries) = fs::read_dir(lib.join("steamapps").join("common")) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() && is_game_folder(&p) {
                if let Some(paks) = find_paks_below(&p, 4) {
                    found.push(paks);
                }
            }
        }
    }
    found
}

pub fn steam_appid(paks: &Path) -> Option<String> {
    let common = paks.ancestors().find(|a| name_lower(a) == "common")?;
    let steamapps = common.parent().filter(|p| name_lower(p) == "steamapps")?;
    let install_dir = paks
        .ancestors()
        .find(|a| a.parent() == Some(common))
        .map(name_lower)?;

    for entry in fs::read_dir(steamapps).ok()?.flatten() {
        let fname = entry.file_name().to_string_lossy().to_string();
        let Some(id) = fname
            .strip_prefix("appmanifest_")
            .and_then(|s| s.strip_suffix(".acf"))
        else {
            continue;
        };
        let Ok(content) = fs::read_to_string(entry.path()) else {
            continue;
        };
        if content
            .lines()
            .filter_map(|l| vdf_value(l, "installdir"))
            .any(|d| d.to_lowercase() == install_dir)
        {
            return Some(id.to_string());
        }
    }
    None
}

#[cfg(windows)]
fn gaming_root_folders(drive: &Path) -> Vec<PathBuf> {
    let Ok(bytes) = fs::read(drive.join(".GamingRoot")) else {
        return vec![];
    };
    if bytes.len() < 8 || &bytes[0..4] != b"RGBX" {
        return vec![];
    }
    let wide: Vec<u16> = bytes[8..]
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    wide.split(|&c| c == 0)
        .filter(|s| !s.is_empty())
        .map(|s| drive.join(String::from_utf16_lossy(s).trim_start_matches('\\')))
        .collect()
}

fn xbox_installs() -> Vec<PathBuf> {
    let mut found = vec![];

    #[cfg(windows)]
    for letter in b'C'..=b'Z' {
        let drive = PathBuf::from(format!("{}:\\", letter as char));
        if !drive.exists() {
            continue;
        }
        let mut roots = gaming_root_folders(&drive);
        roots.push(drive.join("XboxGames"));
        roots.dedup();

        for root in roots {
            let Ok(entries) = fs::read_dir(&root) else {
                continue;
            };
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() && is_game_folder(&p) {
                    if let Some(paks) = find_paks_below(&p, 5) {
                        found.push(paks);
                    }
                }
            }
        }
    }

    found
}

pub fn detect_installs() -> Vec<PathBuf> {
    let mut all = steam_installs();
    for p in xbox_installs() {
        if !all.contains(&p) {
            all.push(p);
        }
    }
    all
}

pub fn find_launch_exe(paks: &Path) -> Option<PathBuf> {
    let root = game_root(paks)?;
    let exes: Vec<PathBuf> = walkdir::WalkDir::new(&root)
        .max_depth(5)
        .into_iter()
        .flatten()
        .map(|e| e.into_path())
        .filter(|p| {
            p.extension()
                .map(|e| e.eq_ignore_ascii_case("exe"))
                .unwrap_or(false)
        })
        .collect();

    let by_name = |pred: &dyn Fn(&str) -> bool| exes.iter().find(|p| pred(&name_lower(p))).cloned();

    by_name(&|n| n == "gamelaunchhelper.exe")
        .or_else(|| by_name(&|n| n.ends_with("-shipping.exe")))
        .or_else(|| {
            by_name(&|n| {
                !n.contains("crash")
                    && !n.contains("unins")
                    && !n.contains("redist")
                    && !n.contains("setup")
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_folder_names() {
        assert!(is_game_folder(Path::new("Minecraft Dungeons II")));
        assert!(is_game_folder(Path::new("MinecraftDungeons2")));
        assert!(is_game_folder(Path::new("Minecraft Dungeons 2")));
        assert!(!is_game_folder(Path::new("Minecraft Dungeons")));
        assert!(!is_game_folder(Path::new("MinecraftDungeons")));
    }

    #[test]
    fn parses_vdf_lines() {
        assert_eq!(
            vdf_value("\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"", "path").as_deref(),
            Some("D:\\SteamLibrary")
        );
        assert_eq!(vdf_value("\"label\"\t\t\"\"", "path"), None);
    }
}
