use std::{
    collections::{
        HashMap,
        HashSet,
    },
    env,
    fs::{
        self,
        File,
    },
    io::Read,
    path::{
        Path,
        PathBuf,
    },
};

use rusqlite::{
    Connection,
    OpenFlags,
};
use serde_json::Value;
use zip::ZipArchive;

use crate::models::{
    FabricStatus,
    InstalledModInfo,
    InstanceInfo,
    LauncherKind,
};

#[derive(Debug, Clone, Default)]
struct ModrinthDbEntry {
    name: Option<String>,
    game_version: Option<String>,
    loader: Option<String>,
    loader_version: Option<String>,
}

pub fn discover_instances() -> Vec<InstanceInfo> {
    let mut found = Vec::new();
    let mut seen = HashSet::new();
    let mut modrinth_map: HashMap<String, ModrinthDbEntry> = HashMap::new();

    if let Some(appdata) = env::var_os("APPDATA").map(PathBuf::from) {
        for app_folder in ["ModrinthApp", "com.modrinth.theseus"] {
            let db_path = appdata.join(app_folder).join("app.db");
            if db_path.is_file() {
                for (k, v) in load_modrinth_db_map(&db_path) {
                    modrinth_map.entry(k).or_insert(v);
                }
            }
        }
    }

    let mut push_if_new = |path: PathBuf, mr_map: &HashMap<String, ModrinthDbEntry>| {
        if !path.is_dir() {
            return;
        }
        let info = inspect_instance_with_modrinth(path, Some(mr_map));
        let key = fs::canonicalize(&info.game_dir)
            .unwrap_or_else(|_| info.game_dir.clone())
            .to_string_lossy()
            .to_ascii_lowercase();
        if seen.insert(key) {
            found.push(info);
        }
    };

    if let Some(appdata) = env::var_os("APPDATA").map(PathBuf::from) {
        let official = appdata.join(".minecraft");
        if official.is_dir() {
            push_if_new(official, &modrinth_map);
        }

        let launcher_roots = [
            appdata.join("PrismLauncher").join("instances"),
            appdata.join("ModrinthApp").join("profiles"),
            appdata.join("com.modrinth.theseus").join("profiles"),
            appdata.join("Polyfrost").join("OneClient").join("clusters"),
            appdata.join("MultiMC").join("instances"),
            appdata.join("ATLauncher").join("instances"),
        ];

        for root in launcher_roots {
            if let Ok(entries) = fs::read_dir(&root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with('_') || name.starts_with('.') {
                            continue;
                        }
                    }
                    push_if_new(path, &modrinth_map);
                }
            }
        }
    }

    if let Some(profile) = env::var_os("USERPROFILE").map(PathBuf::from) {
        let cf_root = profile
            .join("curseforge")
            .join("minecraft")
            .join("Instances");
        if let Ok(entries) = fs::read_dir(&cf_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    push_if_new(path, &modrinth_map);
                }
            }
        }
    }

    disambiguate_instance_names(&mut found);

    found.sort_by(|a, b| {
        b.fabric
            .is_detected()
            .cmp(&a.fabric.is_detected())
            .then_with(|| b.has_installed_mod().cmp(&a.has_installed_mod()))
            .then_with(|| {
                a.display_name
                    .to_ascii_lowercase()
                    .cmp(&b.display_name.to_ascii_lowercase())
            })
    });

    found
}

pub fn disambiguate_instance_names(instances: &mut [InstanceInfo]) {
    let mut counts: HashMap<(String, LauncherKind), usize> = HashMap::new();
    for inst in instances.iter() {
        *counts
            .entry((inst.display_name.to_ascii_lowercase(), inst.launcher))
            .or_insert(0) += 1;
    }

    for inst in instances.iter_mut() {
        let key = (inst.display_name.to_ascii_lowercase(), inst.launcher);
        if counts.get(&key).copied().unwrap_or(0) > 1 {
            if let Some(folder) = inst
                .selected_path
                .file_name()
                .and_then(|n| n.to_str())
                .filter(|f| !f.eq_ignore_ascii_case(&inst.display_name))
            {
                if folder
                    .to_ascii_lowercase()
                    .starts_with(&inst.display_name.to_ascii_lowercase())
                {
                    inst.display_name = folder.to_string();
                } else {
                    inst.display_name = format!("{} ({folder})", inst.display_name);
                }
            }
        }
    }
}

pub fn inspect_instance(selected_path: PathBuf) -> InstanceInfo {
    inspect_instance_with_modrinth(selected_path, None)
}

fn inspect_instance_with_modrinth(
    selected_path: PathBuf,
    cached_modrinth: Option<&HashMap<String, ModrinthDbEntry>>,
) -> InstanceInfo {
    let (instance_root, game_dir) = resolve_instance_paths(&selected_path);
    let launcher = detect_launcher_kind(&instance_root, &game_dir);
    let mods_dir = game_dir.join("mods");

    let local_mr_map;
    let mr_entry = if launcher == LauncherKind::Modrinth {
        if let Some(map) = cached_modrinth {
            lookup_modrinth_entry(&instance_root, map)
        } else if let Some(db_path) = find_modrinth_db_for_instance(&instance_root) {
            local_mr_map = load_modrinth_db_map(&db_path);
            lookup_modrinth_entry(&instance_root, &local_mr_map)
        } else {
            None
        }
    } else {
        None
    };

    let fabric = detect_fabric(&instance_root, &game_dir, mr_entry);
    let installed_mods = find_installed_marlowww_mods(&mods_dir);
    let display_name = build_display_name(&instance_root, &game_dir, launcher, mr_entry);

    InstanceInfo {
        display_name,
        launcher,
        selected_path,
        game_dir,
        mods_dir,
        fabric,
        installed_mods,
    }
}

fn find_modrinth_db_for_instance(instance_root: &Path) -> Option<PathBuf> {
    let profiles_dir = instance_root.parent()?;
    let app_dir = profiles_dir.parent()?;
    let candidate = app_dir.join("app.db");
    if candidate.is_file() {
        return Some(candidate);
    }
    if let Some(appdata) = env::var_os("APPDATA").map(PathBuf::from) {
        let fallback = appdata.join("ModrinthApp").join("app.db");
        if fallback.is_file() {
            return Some(fallback);
        }
    }
    None
}

fn lookup_modrinth_entry<'a>(
    instance_root: &Path,
    map: &'a HashMap<String, ModrinthDbEntry>,
) -> Option<&'a ModrinthDbEntry> {
    let folder = instance_root.file_name()?.to_str()?;
    map.get(&folder.to_ascii_lowercase())
}

fn load_modrinth_db_map(db_path: &Path) -> HashMap<String, ModrinthDbEntry> {
    let mut out = HashMap::new();
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let Ok(conn) = Connection::open_with_flags(db_path, flags) else {
        return out;
    };
    let _ = conn.busy_timeout(std::time::Duration::from_millis(250));

    let query = "SELECT i.path, i.name, c.game_version, c.loader, c.loader_version \
                 FROM instances i \
                 LEFT JOIN instance_content_sets c ON i.applied_content_set_id = c.id";

    let Ok(mut stmt) = conn.prepare(query) else {
        return out;
    };

    let rows = stmt.query_map([], |row| {
        let raw_path: String = row.get(0)?;
        let name: Option<String> = row.get(1).ok();
        let game_version: Option<String> = row.get(2).ok();
        let loader: Option<String> = row.get(3).ok();
        let loader_version: Option<String> = row.get(4).ok();
        Ok((
            raw_path,
            ModrinthDbEntry {
                name,
                game_version,
                loader,
                loader_version,
            },
        ))
    });

    if let Ok(iter) = rows {
        for item in iter.flatten() {
            let (raw_path, entry) = item;
            let trimmed = raw_path
                .trim_matches(|c| c == '/' || c == '\\')
                .to_string();
            if !trimmed.is_empty() {
                out.insert(trimmed.to_ascii_lowercase(), entry.clone());
                if let Some(base) = Path::new(&trimmed).file_name().and_then(|n| n.to_str()) {
                    out.entry(base.to_ascii_lowercase()).or_insert(entry);
                }
            }
        }
    }

    out
}

fn resolve_instance_paths(selected: &Path) -> (PathBuf, PathBuf) {
    let is_mods_folder = selected
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("mods"));

    if is_mods_folder {
        if let Some(parent) = selected.parent() {
            if looks_like_game_dir(parent) {
                let root = parent
                    .parent()
                    .filter(|gp| {
                        gp.join("mmc-pack.json").is_file() || gp.join("instance.cfg").is_file()
                    })
                    .unwrap_or(parent);
                return (root.to_path_buf(), parent.to_path_buf());
            }
        }
    }

    if selected.join("mmc-pack.json").is_file() || selected.join("instance.cfg").is_file() {
        let dot_mc = selected.join(".minecraft");
        if dot_mc.is_dir() {
            return (selected.to_path_buf(), dot_mc);
        }
        let plain_mc = selected.join("minecraft");
        if plain_mc.is_dir() {
            return (selected.to_path_buf(), plain_mc);
        }
    }

    if !looks_like_game_dir(selected) {
        let dot_mc = selected.join(".minecraft");
        if dot_mc.is_dir() {
            return (selected.to_path_buf(), dot_mc);
        }
        let plain_mc = selected.join("minecraft");
        if plain_mc.is_dir() && looks_like_game_dir(&plain_mc) {
            return (selected.to_path_buf(), plain_mc);
        }
    }

    if let Some(parent) = selected.parent() {
        let name_is_mc = selected.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
            n.eq_ignore_ascii_case(".minecraft") || n.eq_ignore_ascii_case("minecraft")
        });
        if name_is_mc
            && (parent.join("mmc-pack.json").is_file() || parent.join("instance.cfg").is_file())
        {
            return (parent.to_path_buf(), selected.to_path_buf());
        }
    }

    (selected.to_path_buf(), selected.to_path_buf())
}

fn looks_like_game_dir(dir: &Path) -> bool {
    dir.join("mods").is_dir()
        || dir.join("versions").is_dir()
        || dir.join("config").is_dir()
        || dir.join(".fabric").is_dir()
        || dir.join(".instance.json").is_file()
        || dir.join("options.txt").is_file()
        || dir.join("launcher_profiles.json").is_file()
        || dir.join("profile.json").is_file()
        || dir.join("minecraftinstance.json").is_file()
        || dir.join("instance.json").is_file()
}

fn detect_launcher_kind(instance_root: &Path, game_dir: &Path) -> LauncherKind {
    let path_str = instance_root.to_string_lossy().to_ascii_lowercase();
    if path_str.contains("oneclient")
        || instance_root.join(".instance.json").is_file()
        || game_dir.join(".instance.json").is_file()
    {
        return LauncherKind::OneClient;
    }
    if path_str.contains("prismlauncher") {
        return LauncherKind::Prism;
    }
    if path_str.contains("multimc") {
        return LauncherKind::MultiMc;
    }
    if path_str.contains("modrinthapp")
        || path_str.contains("com.modrinth.theseus")
        || find_modrinth_db_for_instance(instance_root).is_some_and(|db| {
            instance_root
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case("profiles"))
                && db.parent() == instance_root.parent().and_then(|p| p.parent())
        })
    {
        return LauncherKind::Modrinth;
    }
    if path_str.contains("curseforge") || game_dir.join("minecraftinstance.json").is_file() {
        return LauncherKind::CurseForge;
    }
    if path_str.contains("atlauncher") || instance_root.join("instance.json").is_file() {
        return LauncherKind::AtLauncher;
    }
    if instance_root.join("mmc-pack.json").is_file()
        || instance_root.join("instance.cfg").is_file()
    {
        return LauncherKind::Prism;
    }
    if game_dir.join("profile.json").is_file() {
        return LauncherKind::Modrinth;
    }
    if game_dir.join("launcher_profiles.json").is_file()
        || game_dir
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case(".minecraft"))
    {
        return LauncherKind::Official;
    }
    LauncherKind::Custom
}

fn build_display_name(
    instance_root: &Path,
    game_dir: &Path,
    launcher: LauncherKind,
    mr_entry: Option<&ModrinthDbEntry>,
) -> String {
    if let Some(mr) = mr_entry {
        if let Some(name) = mr.name.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            return name.to_string();
        }
    }
    if let Some(oc_name) = read_oneclient_name(&instance_root.join(".instance.json"))
        .or_else(|| read_oneclient_name(&game_dir.join(".instance.json")))
    {
        return oc_name;
    }
    if let Some(mr_json_name) = read_modrinth_json_name(&game_dir.join("profile.json")) {
        return mr_json_name;
    }
    if let Some(cfg_name) = read_instance_cfg_name(instance_root) {
        return cfg_name;
    }
    if launcher == LauncherKind::Official
        && game_dir
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case(".minecraft"))
    {
        return ".minecraft".to_string();
    }
    instance_root
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("Minecraft Instance")
        .to_string()
}

fn read_oneclient_name(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let json: Value = serde_json::from_str(&text).ok()?;
    json.get("name")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

fn read_modrinth_json_name(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let json: Value = serde_json::from_str(&text).ok()?;
    json.get("metadata")
        .and_then(|m| m.get("name"))
        .or_else(|| json.get("name"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

fn read_instance_cfg_name(instance_root: &Path) -> Option<String> {
    let content = fs::read_to_string(instance_root.join("instance.cfg")).ok()?;
    for line in content.lines() {
        if let Some(val) = line.strip_prefix("name=") {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn detect_fabric(
    instance_root: &Path,
    game_dir: &Path,
    mr_entry: Option<&ModrinthDbEntry>,
) -> FabricStatus {
    if let Some(status) = check_oneclient_instance(&instance_root.join(".instance.json"))
        .or_else(|| check_oneclient_instance(&game_dir.join(".instance.json")))
    {
        return status;
    }

    if let Some(mr) = mr_entry {
        if let Some(status) = check_modrinth_db_entry(mr) {
            return status;
        }
    }

    if let Some(status) = check_mmc_pack(&instance_root.join("mmc-pack.json")) {
        return status;
    }

    if let Some(status) = check_modrinth_profile(&game_dir.join("profile.json")) {
        return status;
    }

    if let Some(status) = check_curseforge_instance(&game_dir.join("minecraftinstance.json")) {
        return status;
    }

    if let Some(status) = check_atlauncher_instance(&instance_root.join("instance.json")) {
        return status;
    }

    if let Some(status) = check_vanilla_versions(game_dir) {
        return status;
    }

    if let Some(status) = check_launcher_profiles(&game_dir.join("launcher_profiles.json")) {
        return status;
    }

    if game_dir.join(".fabric").is_dir() {
        return FabricStatus::Detected {
            loader_version: None,
            mc_versions: Vec::new(),
            source: ".fabric folder".to_string(),
        };
    }

    if has_fabric_api_in_mods(&game_dir.join("mods")) {
        return FabricStatus::Detected {
            loader_version: None,
            mc_versions: Vec::new(),
            source: "Fabric API in mods/".to_string(),
        };
    }

    FabricStatus::Missing {
        reason: "Fabric Loader was not found in this folder. Install Fabric Loader on this instance first."
            .to_string(),
    }
}

fn check_oneclient_instance(path: &Path) -> Option<FabricStatus> {
    let text = fs::read_to_string(path).ok()?;
    let json: Value = serde_json::from_str(&text).ok()?;
    let loader = json
        .get("mc_loader")
        .or_else(|| json.get("loader"))
        .and_then(|v| v.as_str())?;
    let mc_ver = json
        .get("mc_version")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);
    let loader_ver = json
        .get("mc_loader_version")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);

    if loader.eq_ignore_ascii_case("fabric") {
        Some(FabricStatus::Detected {
            loader_version: loader_ver,
            mc_versions: mc_ver.into_iter().collect(),
            source: ".instance.json".to_string(),
        })
    } else {
        Some(FabricStatus::Missing {
            reason: format!("This OneClient cluster runs {loader} instead of Fabric Loader."),
        })
    }
}

fn check_modrinth_db_entry(entry: &ModrinthDbEntry) -> Option<FabricStatus> {
    let loader = entry.loader.as_deref()?.trim();
    if loader.is_empty() {
        return None;
    }
    let mc_versions: Vec<String> = entry
        .game_version
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .into_iter()
        .collect();
    let loader_version = entry
        .loader_version
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);

    if loader.eq_ignore_ascii_case("fabric") {
        Some(FabricStatus::Detected {
            loader_version,
            mc_versions,
            source: "Modrinth app.db".to_string(),
        })
    } else {
        Some(FabricStatus::Missing {
            reason: format!("This Modrinth profile runs {loader} instead of Fabric Loader."),
        })
    }
}

fn check_mmc_pack(path: &Path) -> Option<FabricStatus> {
    let text = fs::read_to_string(path).ok()?;
    let json: Value = serde_json::from_str(&text).ok()?;
    let components = json.get("components")?.as_array()?;

    let mut fabric_ver: Option<String> = None;
    let mut mc_ver: Option<String> = None;
    let mut other_loader: Option<&str> = None;

    for comp in components {
        let uid = comp.get("uid").and_then(|v| v.as_str()).unwrap_or("");
        let version = comp
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        match uid {
            "net.fabricmc.fabric-loader" => {
                fabric_ver = version.or_else(|| Some("installed".to_string()))
            }
            "net.minecraft" => mc_ver = version,
            "net.minecraftforge" => other_loader = Some("Forge"),
            "net.neoforged" => other_loader = Some("NeoForge"),
            "org.quiltmc.quilt-loader" => other_loader = Some("Quilt"),
            _ => {}
        }
    }

    if let Some(loader) = fabric_ver {
        let loader_version = if loader == "installed" {
            None
        } else {
            Some(loader)
        };
        return Some(FabricStatus::Detected {
            loader_version,
            mc_versions: mc_ver.into_iter().collect(),
            source: "mmc-pack.json".to_string(),
        });
    }

    let reason = if let Some(other) = other_loader {
        format!("This instance runs {other}. Marlowww Client requires Fabric Loader.")
    } else {
        "Fabric Loader is not installed on this Prism/MultiMC instance.".to_string()
    };
    Some(FabricStatus::Missing { reason })
}

fn check_modrinth_profile(path: &Path) -> Option<FabricStatus> {
    let text = fs::read_to_string(path).ok()?;
    let json: Value = serde_json::from_str(&text).ok()?;
    let loader = json
        .get("loader")
        .or_else(|| json.get("mod_loader"))
        .or_else(|| json.get("metadata").and_then(|m| m.get("loader")))
        .and_then(|v| v.as_str())?;
    let game_ver = json
        .get("game_version")
        .or_else(|| json.get("metadata").and_then(|m| m.get("game_version")))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let loader_ver = json
        .get("loader_version")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if loader.eq_ignore_ascii_case("fabric") {
        Some(FabricStatus::Detected {
            loader_version: loader_ver,
            mc_versions: game_ver.into_iter().collect(),
            source: "profile.json".to_string(),
        })
    } else {
        Some(FabricStatus::Missing {
            reason: format!("This Modrinth profile runs {loader} instead of Fabric Loader."),
        })
    }
}

fn check_curseforge_instance(path: &Path) -> Option<FabricStatus> {
    let text = fs::read_to_string(path).ok()?;
    let json: Value = serde_json::from_str(&text).ok()?;
    let base_loader = json.get("baseModLoader");
    let loader_name = base_loader
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let game_ver = json
        .get("gameVersion")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if loader_name.to_ascii_lowercase().contains("fabric") {
        let loader_ver = base_loader
            .and_then(|v| v.get("forgeVersion"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        Some(FabricStatus::Detected {
            loader_version: loader_ver,
            mc_versions: game_ver.into_iter().collect(),
            source: "minecraftinstance.json".to_string(),
        })
    } else if !loader_name.is_empty() {
        Some(FabricStatus::Missing {
            reason: format!(
                "This CurseForge instance runs {loader_name} instead of Fabric Loader."
            ),
        })
    } else {
        None
    }
}

fn check_atlauncher_instance(path: &Path) -> Option<FabricStatus> {
    let text = fs::read_to_string(path).ok()?;
    let json: Value = serde_json::from_str(&text).ok()?;
    let loader_obj = json.get("launcher")?.get("loaderVersion")?;
    let loader_type = loader_obj.get("type").and_then(|v| v.as_str())?;
    let mc_ver = json
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let loader_ver = loader_obj
        .get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if loader_type.eq_ignore_ascii_case("fabric") {
        Some(FabricStatus::Detected {
            loader_version: loader_ver,
            mc_versions: mc_ver.into_iter().collect(),
            source: "instance.json".to_string(),
        })
    } else {
        Some(FabricStatus::Missing {
            reason: format!(
                "This ATLauncher instance runs {loader_type} instead of Fabric Loader."
            ),
        })
    }
}

fn check_vanilla_versions(game_dir: &Path) -> Option<FabricStatus> {
    let versions_dir = game_dir.join("versions");
    let entries = fs::read_dir(&versions_dir).ok()?;

    let mut loader_version: Option<String> = None;
    let mut mc_versions: Vec<String> = Vec::new();
    let mut detected = false;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if lower.contains("fabric-loader") {
            detected = true;
            if let Some((lv, mv)) = parse_fabric_version_folder(name) {
                if loader_version.is_none() {
                    loader_version = Some(lv);
                }
                if !mc_versions.contains(&mv) {
                    mc_versions.push(mv);
                }
            }
        } else {
            let json_path = path.join(format!("{name}.json"));
            if let Ok(content) = fs::read_to_string(&json_path) {
                if content.contains("net.fabricmc:fabric-loader") {
                    detected = true;
                }
            }
        }
    }

    if detected {
        mc_versions.sort_by(|a, b| crate::models::compare_versions_desc(a, b));
        Some(FabricStatus::Detected {
            loader_version,
            mc_versions,
            source: "versions/".to_string(),
        })
    } else {
        None
    }
}

fn parse_fabric_version_folder(name: &str) -> Option<(String, String)> {
    let rest = name
        .strip_prefix("fabric-loader-")
        .or_else(|| name.split("fabric-loader-").nth(1))?;
    let (loader, mc) = rest.split_once('-')?;
    if !loader.is_empty() && !mc.is_empty() {
        Some((loader.to_string(), mc.to_string()))
    } else {
        None
    }
}

fn check_launcher_profiles(path: &Path) -> Option<FabricStatus> {
    let content = fs::read_to_string(path).ok()?;
    let json: Value = serde_json::from_str(&content).ok()?;
    let profiles = json.get("profiles")?.as_object()?;
    let mut mc_versions = Vec::new();
    let mut loader_version = None;
    let mut detected = false;

    for profile in profiles.values() {
        let version_id = profile
            .get("lastVersionId")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if version_id.to_ascii_lowercase().contains("fabric-loader") {
            detected = true;
            if let Some((lv, mv)) = parse_fabric_version_folder(version_id) {
                if loader_version.is_none() {
                    loader_version = Some(lv);
                }
                if !mc_versions.contains(&mv) {
                    mc_versions.push(mv);
                }
            }
        }
    }

    if detected {
        Some(FabricStatus::Detected {
            loader_version,
            mc_versions,
            source: "launcher_profiles.json".to_string(),
        })
    } else {
        None
    }
}

fn has_fabric_api_in_mods(mods_dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(mods_dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(|s| s.to_ascii_lowercase()) else {
            continue;
        };
        if name.ends_with(".jar")
            && (name.starts_with("fabric-api") || name.contains("fabric-api-"))
        {
            return true;
        }
    }
    false
}

pub fn is_marlowww_jar_name(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    lower.ends_with(".jar")
        && (lower.contains("marlowwwclient")
            || lower.contains("marlowclient")
            || lower.contains("im-not-cheating-you-are"))
}

pub fn inspect_marlowww_jar_file(path: &Path) -> Option<InstalledModInfo> {
    let file_name = path.file_name()?.to_str()?.to_string();
    let lower = file_name.to_ascii_lowercase();
    if !lower.ends_with(".jar") {
        return None;
    }
    let is_android = lower.contains("-android");

    if let Ok(file) = File::open(path) {
        if let Ok(mut archive) = ZipArchive::new(file) {
            if let Ok(mut mod_json_file) = archive.by_name("fabric.mod.json") {
                let mut raw = String::new();
                if mod_json_file.read_to_string(&mut raw).is_ok() {
                    if let Ok(json) = serde_json::from_str::<Value>(&raw) {
                        let id = json
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        if id == "imnotcheatingyouare"
                            || id == "marlowwwclient"
                            || id == "marlowclient"
                        {
                            let version = json
                                .get("version")
                                .and_then(|v| v.as_str())
                                .map(String::from);
                            let display_name =
                                json.get("name").and_then(|v| v.as_str()).map(String::from);
                            return Some(InstalledModInfo {
                                file_name,
                                version,
                                display_name,
                                is_android,
                            });
                        } else if !id.is_empty() {
                            return None;
                        }
                    }
                }
            }
        }
    }

    if is_marlowww_jar_name(&file_name) {
        Some(InstalledModInfo {
            file_name,
            version: None,
            display_name: None,
            is_android,
        })
    } else {
        None
    }
}

pub fn find_installed_marlowww_mods(mods_dir: &Path) -> Vec<InstalledModInfo> {
    let Ok(entries) = fs::read_dir(mods_dir) else {
        return Vec::new();
    };
    let mut mods = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(info) = inspect_marlowww_jar_file(&path) {
            mods.push(info);
        }
    }
    mods.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    mods
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use zip::{
        CompressionMethod,
        ZipWriter,
        write::SimpleFileOptions,
    };

    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!(
            "marlowww_launcher_test_{}_{}",
            label,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_test_jar(path: &Path, mod_id: &str, mod_name: &str, mod_ver: &str) {
        let file = File::create(path).unwrap();
        let mut zip = ZipWriter::new(file);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file("fabric.mod.json", opts).unwrap();
        let body = format!(
            r#"{{"schemaVersion":1,"id":"{mod_id}","version":"{mod_ver}","name":"{mod_name}"}}"#
        );
        zip.write_all(body.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn detects_prism_fabric_and_resolves_dot_minecraft() {
        let root = temp_dir("prism_fabric");
        fs::create_dir_all(root.join(".minecraft").join("mods")).unwrap();
        fs::write(
            root.join("mmc-pack.json"),
            r#"{"components":[{"uid":"net.minecraft","version":"1.21.11"},{"uid":"net.fabricmc.fabric-loader","version":"0.16.10"}]}"#,
        )
        .unwrap();
        fs::write(root.join("instance.cfg"), "name=PvP 1.21.11\n").unwrap();

        let info = inspect_instance(root.clone());
        assert_eq!(info.display_name, "PvP 1.21.11");
        assert_eq!(info.game_dir, root.join(".minecraft"));
        assert_eq!(info.mods_dir, root.join(".minecraft").join("mods"));
        assert!(info.fabric.is_detected());
        assert_eq!(info.fabric.mc_versions(), &["1.21.11".to_string()]);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn detects_oneclient_cluster_json() {
        let root = temp_dir("oneclient_cluster");
        let cluster = root.join("Polyfrost").join("OneClient").join("clusters").join("26.3 Fabric");
        fs::create_dir_all(cluster.join("mods")).unwrap();
        fs::write(
            cluster.join(".instance.json"),
            r#"{"name":"26.3 Fabric","mc_version":"26.3","mc_loader":"Fabric","mc_loader_version":null,"kind":"one_client"}"#,
        )
        .unwrap();

        let info = inspect_instance(cluster.clone());
        assert_eq!(info.launcher, LauncherKind::OneClient);
        assert_eq!(info.display_name, "26.3 Fabric");
        assert!(info.fabric.is_detected());
        assert_eq!(info.fabric.mc_versions(), &["26.3".to_string()]);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn detects_modrinth_sqlite_app_db() {
        let root = temp_dir("modrinth_sqlite");
        let app_dir = root.join("ModrinthApp");
        let profile_dir = app_dir.join("profiles").join("Remarkably Optimized (1)");
        fs::create_dir_all(profile_dir.join("mods")).unwrap();

        let db_path = app_dir.join("app.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE instances (id TEXT PRIMARY KEY, path TEXT NOT NULL, applied_content_set_id TEXT, name TEXT);
             CREATE TABLE instance_content_sets (id TEXT PRIMARY KEY, instance_id TEXT, game_version TEXT, loader TEXT, loader_version TEXT);
             INSERT INTO instance_content_sets VALUES ('cs1', 'i1', '1.21.11', 'fabric', '0.16.12');
             INSERT INTO instances VALUES ('i1', 'Remarkably Optimized (1)', 'cs1', 'Insanely Optimized PvP');",
        )
        .unwrap();
        drop(conn);

        let info = inspect_instance(profile_dir);
        assert_eq!(info.launcher, LauncherKind::Modrinth);
        assert_eq!(info.display_name, "Insanely Optimized PvP");
        assert!(info.fabric.is_detected());
        assert_eq!(info.fabric.mc_versions(), &["1.21.11".to_string()]);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn detects_stealth_renamed_marlowww_jar_and_ignores_unrelated_marlow_mod() {
        let root = temp_dir("stealth_jar");
        let mods = root.join("mods");
        fs::create_dir_all(&mods).unwrap();

        write_test_jar(
            &mods.join("sodium-extra.jar"),
            "imnotcheatingyouare",
            "Sodium Extra",
            "5.2.1",
        );
        write_test_jar(
            &mods.join("Marlow Crystal Optimizer.jar"),
            "marlow-crystal-optimizer",
            "Marlow Crystal Optimizer",
            "1.0.0",
        );

        let detected = find_installed_marlowww_mods(&mods);
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].file_name, "sodium-extra.jar");
        assert_eq!(detected[0].version.as_deref(), Some("5.2.1"));
        assert_eq!(detected[0].custom_ghost_name(), Some("Sodium Extra"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn disambiguates_duplicate_prism_instance_names() {
        let mut list = vec![
            InstanceInfo {
                display_name: "1.21.11".to_string(),
                launcher: LauncherKind::Prism,
                selected_path: PathBuf::from(r"C:\Prism\instances\1.21.11"),
                game_dir: PathBuf::from(r"C:\Prism\instances\1.21.11\.minecraft"),
                mods_dir: PathBuf::from(r"C:\Prism\instances\1.21.11\.minecraft\mods"),
                fabric: FabricStatus::Missing {
                    reason: String::new(),
                },
                installed_mods: Vec::new(),
            },
            InstanceInfo {
                display_name: "1.21.11".to_string(),
                launcher: LauncherKind::Prism,
                selected_path: PathBuf::from(r"C:\Prism\instances\1.21.11(1)"),
                game_dir: PathBuf::from(r"C:\Prism\instances\1.21.11(1)\.minecraft"),
                mods_dir: PathBuf::from(r"C:\Prism\instances\1.21.11(1)\.minecraft\mods"),
                fabric: FabricStatus::Missing {
                    reason: String::new(),
                },
                installed_mods: Vec::new(),
            },
        ];
        disambiguate_instance_names(&mut list);
        assert_eq!(list[0].display_name, "1.21.11");
        assert_eq!(list[1].display_name, "1.21.11(1)");
    }
}
