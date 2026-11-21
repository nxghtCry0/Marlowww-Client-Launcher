use std::{
    fs::{
        self,
        File,
    },
    io::{
        Read,
        Write,
    },
    path::{
        Path,
        PathBuf,
    },
    time::{
        Duration,
        Instant,
    },
};

use async_channel::Sender;
use serde::Deserialize;
use serde_json::Value;
use zip::{
    CompressionMethod,
    ZipArchive,
    ZipWriter,
    write::SimpleFileOptions,
};

use crate::{
    minecraft::inspect_marlowww_jar_file,
    models::{
        DownloadUpdate,
        PlatformKind,
        ReleaseAsset,
        ReleaseInfo,
        effective_in_game_name,
        sanitize_jar_filename,
    },
};

pub const RELEASES_API_URL: &str =
    "https://api.github.com/repos/nxghtCry0/marlowwwclient/releases?per_page=100";
const USER_AGENT: &str = "marlowww-client-launcher/0.2.0";

#[derive(Deserialize)]
struct ApiRelease {
    id: u64,
    tag_name: String,
    name: Option<String>,
    prerelease: bool,
    draft: bool,
    published_at: Option<String>,
    html_url: String,
    assets: Vec<ApiAsset>,
}

#[derive(Deserialize)]
struct ApiAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

pub fn fetch_releases() -> Result<Vec<ReleaseInfo>, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(20))
        .build();

    let response = agent
        .get(RELEASES_API_URL)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|err| format!("Failed to reach GitHub releases: {err}"))?;

    let api_releases: Vec<ApiRelease> = response
        .into_json()
        .map_err(|err| format!("Failed to parse GitHub releases JSON: {err}"))?;

    let mut releases = Vec::new();
    for raw in api_releases {
        if raw.draft {
            continue;
        }
        let mut assets = Vec::new();
        for asset in raw.assets {
            if !is_installable_jar(&asset.name) {
                continue;
            }
            let (platform, mc_version, is_explicit_mc_version) =
                parse_asset_metadata(&asset.name);
            assets.push(ReleaseAsset {
                name: asset.name,
                download_url: asset.browser_download_url,
                size: asset.size,
                platform,
                mc_version,
                is_explicit_mc_version,
            });
        }
        if assets.is_empty() {
            continue;
        }
        let published_date = raw
            .published_at
            .as_deref()
            .and_then(|s| s.split('T').next())
            .unwrap_or("")
            .to_string();

        releases.push(ReleaseInfo {
            id: raw.id,
            tag_name: raw.tag_name.clone(),
            name: raw.name.unwrap_or_else(|| raw.tag_name.clone()),
            published_date,
            prerelease: raw.prerelease,
            html_url: raw.html_url,
            assets,
        });
    }

    if releases.is_empty() {
        return Err("No downloadable release JAR files were found on GitHub.".to_string());
    }

    Ok(releases)
}

pub fn is_installable_jar(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    if !lower.ends_with(".jar") {
        return false;
    }
    let blocked = [
        "-sources.jar",
        "-javadoc.jar",
        "-dev.jar",
        "-dev-shadow.jar",
        "-slim.jar",
    ];
    !blocked.iter().any(|suffix| lower.ends_with(suffix))
}

pub fn parse_asset_metadata(file_name: &str) -> (PlatformKind, String, bool) {
    let without_ext = file_name
        .strip_suffix(".jar")
        .or_else(|| file_name.strip_suffix(".JAR"))
        .unwrap_or(file_name);

    let lower = without_ext.to_ascii_lowercase();
    let (platform, stem) = if let Some(s) = strip_ci_suffix(without_ext, &lower, "-android") {
        (PlatformKind::Android, s)
    } else if let Some(s) = strip_ci_suffix(without_ext, &lower, "-linux") {
        (PlatformKind::Linux, s)
    } else if let Some(s) = strip_ci_suffix(without_ext, &lower, "-macos")
        .or_else(|| strip_ci_suffix(without_ext, &lower, "-mac"))
        .or_else(|| strip_ci_suffix(without_ext, &lower, "-osx"))
    {
        (PlatformKind::MacOS, s)
    } else if let Some(s) = strip_ci_suffix(without_ext, &lower, "-windows")
        .or_else(|| strip_ci_suffix(without_ext, &lower, "-win"))
    {
        (PlatformKind::Desktop, s)
    } else {
        (PlatformKind::Desktop, without_ext)
    };

    if let Some((_before, after)) = stem.rsplit_once("+mc") {
        if looks_like_mc_version(after) {
            return (platform, after.to_string(), true);
        }
    }

    let parts: Vec<&str> = stem.split('-').filter(|p| !p.is_empty()).collect();
    if parts.len() >= 2 {
        let last = parts[parts.len() - 1];
        let prev = parts[parts.len() - 2];

        if let Some(mc) = last
            .strip_prefix("mc")
            .or_else(|| last.strip_prefix("MC"))
            .filter(|v| looks_like_mc_version(v))
        {
            return (platform, mc.to_string(), true);
        }

        if looks_like_mc_version(last) && looks_like_mod_version(prev) {
            return (platform, last.to_string(), true);
        }
    }

    (platform, "Default".to_string(), false)
}

fn strip_ci_suffix<'a>(original: &'a str, lower: &str, suffix: &str) -> Option<&'a str> {
    if lower.ends_with(suffix) {
        Some(&original[..original.len() - suffix.len()])
    } else {
        None
    }
}

fn looks_like_mod_version(segment: &str) -> bool {
    let trimmed = segment
        .strip_prefix('v')
        .or_else(|| segment.strip_prefix('V'))
        .unwrap_or(segment);
    !trimmed.is_empty() && trimmed.chars().next().is_some_and(|c| c.is_ascii_digit())
}

fn looks_like_mc_version(segment: &str) -> bool {
    let parts: Vec<&str> = segment.split('.').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return false;
    }
    if !parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())) {
        return false;
    }
    let major: u32 = parts[0].parse().unwrap_or(0);
    major == 1 || (20..=99).contains(&major)
}

pub fn rewrite_jar_display_name(jar_path: &Path, custom_display_name: &str) -> Result<(), String> {
    let trimmed = custom_display_name.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let src_file = File::open(jar_path)
        .map_err(|err| format!("Failed to open downloaded JAR for renaming: {err}"))?;
    let mut archive = ZipArchive::new(src_file)
        .map_err(|err| format!("Downloaded file is not a valid JAR archive: {err}"))?;

    let repack_path = jar_path.with_extension("repack.part");
    let out_file = File::create(&repack_path)
        .map_err(|err| format!("Failed to create temporary JAR during rename: {err}"))?;
    let mut writer = ZipWriter::new(out_file);
    let mut patched_mod_json = false;

    for idx in 0..archive.len() {
        let is_fabric_mod_json = {
            let probe = archive
                .by_index_raw(idx)
                .map_err(|err| format!("Failed reading JAR entry #{idx}: {err}"))?;
            probe
                .name()
                .map(|n| n == "fabric.mod.json")
                .unwrap_or(false)
        };

        if is_fabric_mod_json {
            let mut entry = archive
                .by_index(idx)
                .map_err(|err| format!("Failed reading fabric.mod.json: {err}"))?;
            let mut raw = String::new();
            entry
                .read_to_string(&mut raw)
                .map_err(|err| format!("Failed decoding fabric.mod.json: {err}"))?;

            let mut json: Value = serde_json::from_str(&raw)
                .map_err(|err| format!("Invalid JSON in fabric.mod.json: {err}"))?;
            if let Some(obj) = json.as_object_mut() {
                obj.insert("name".to_string(), Value::String(trimmed.to_string()));
                if let Some(desc) = obj.get("description").and_then(|v| v.as_str()) {
                    let lower_desc = desc.to_ascii_lowercase();
                    if lower_desc.contains("marlow") || lower_desc.contains("ghost client") {
                        obj.insert(
                            "description".to_string(),
                            Value::String(format!("{trimmed} mod for Fabric.")),
                        );
                    }
                }
            }
            let patched = serde_json::to_vec_pretty(&json)
                .map_err(|err| format!("Failed serialising fabric.mod.json: {err}"))?;
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            writer
                .start_file("fabric.mod.json", opts)
                .map_err(|err| format!("Failed writing fabric.mod.json: {err}"))?;
            writer
                .write_all(&patched)
                .map_err(|err| format!("Failed saving fabric.mod.json: {err}"))?;
            patched_mod_json = true;
        } else {
            let raw_entry = archive
                .by_index_raw(idx)
                .map_err(|err| format!("Failed reading raw JAR entry #{idx}: {err}"))?;
            writer
                .raw_copy_file(raw_entry)
                .map_err(|err| format!("Failed copying JAR entry #{idx}: {err}"))?;
        }
    }

    if !patched_mod_json {
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let fallback_json = serde_json::json!({
            "schemaVersion": 1,
            "id": "imnotcheatingyouare",
            "version": "1.0.0",
            "name": trimmed,
        });
        let bytes = serde_json::to_vec_pretty(&fallback_json)
            .map_err(|err| format!("Failed creating fabric.mod.json: {err}"))?;
        writer
            .start_file("fabric.mod.json", opts)
            .map_err(|err| format!("Failed adding fabric.mod.json: {err}"))?;
        writer
            .write_all(&bytes)
            .map_err(|err| format!("Failed writing fabric.mod.json: {err}"))?;
    }

    writer
        .finish()
        .map_err(|err| format!("Failed finalising renamed JAR: {err}"))?;
    drop(archive);

    let _ = fs::remove_file(jar_path);
    fs::rename(&repack_path, jar_path).map_err(|err| {
        let _ = fs::remove_file(&repack_path);
        format!("Failed replacing JAR with renamed archive: {err}")
    })?;

    Ok(())
}

pub fn download_and_install(
    asset: ReleaseAsset,
    mods_dir: PathBuf,
    custom_name: Option<String>,
    replace_existing: bool,
    known_installed_files: Vec<String>,
    tx: Sender<DownloadUpdate>,
) {
    let result = perform_download_and_install(
        &asset,
        &mods_dir,
        custom_name.as_deref(),
        replace_existing,
        &known_installed_files,
        &tx,
    );
    match result {
        Ok((file_name, in_game_name, dest_path, removed_old)) => {
            let _ = tx.send_blocking(DownloadUpdate::Done {
                file_name,
                in_game_name,
                dest_path,
                removed_old,
            });
        }
        Err(err) => {
            let _ = tx.send_blocking(DownloadUpdate::Failed(err));
        }
    }
}

fn perform_download_and_install(
    asset: &ReleaseAsset,
    mods_dir: &PathBuf,
    custom_name: Option<&str>,
    replace_existing: bool,
    known_installed_files: &[String],
    tx: &Sender<DownloadUpdate>,
) -> Result<(String, String, PathBuf, Vec<String>), String> {
    fs::create_dir_all(mods_dir)
        .map_err(|err| format!("Failed to create mods folder: {err}"))?;

    let raw_custom = custom_name.unwrap_or("").trim();
    let target_file_name = sanitize_jar_filename(raw_custom, &asset.name);
    let in_game_name = effective_in_game_name(raw_custom);

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(60))
        .build();

    let response = agent
        .get(&asset.download_url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|err| format!("Failed to download {}: {err}", asset.name))?;

    let total = response
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(asset.size);

    let temp_path = mods_dir.join(format!("{target_file_name}.part"));
    let mut temp_file = File::create(&temp_path)
        .map_err(|err| format!("Failed to create temporary file: {err}"))?;

    let mut reader = response.into_reader();
    let mut buffer = [0u8; 64 * 1024];
    let mut downloaded: u64 = 0;
    let started = Instant::now();
    let mut last_emit = Instant::now();

    let _ = tx.send_blocking(DownloadUpdate::Progress {
        downloaded: 0,
        total,
        speed_bps: 0.0,
    });

    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                if let Err(err) = temp_file.write_all(&buffer[..n]) {
                    let _ = fs::remove_file(&temp_path);
                    return Err(format!("Failed while writing JAR file: {err}"));
                }
                downloaded += n as u64;
                if last_emit.elapsed() >= Duration::from_millis(35) {
                    let elapsed = started.elapsed().as_secs_f64().max(0.001);
                    let _ = tx.send_blocking(DownloadUpdate::Progress {
                        downloaded,
                        total: total.max(downloaded),
                        speed_bps: downloaded as f64 / elapsed,
                    });
                    last_emit = Instant::now();
                }
            }
            Err(err) => {
                let _ = fs::remove_file(&temp_path);
                return Err(format!("Download interrupted: {err}"));
            }
        }
    }

    if let Err(err) = temp_file.flush() {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("Failed to flush downloaded file: {err}"))?;
    }
    drop(temp_file);

    let elapsed = started.elapsed().as_secs_f64().max(0.001);
    let _ = tx.send_blocking(DownloadUpdate::Progress {
        downloaded,
        total: total.max(downloaded),
        speed_bps: downloaded as f64 / elapsed,
    });
    let _ = tx.send_blocking(DownloadUpdate::Finishing);

    if !raw_custom.is_empty() {
        if let Err(err) = rewrite_jar_display_name(&temp_path, &in_game_name) {
            let _ = fs::remove_file(&temp_path);
            return Err(err);
        }
    }

    let mut removed_old = Vec::new();
    if replace_existing {
        if let Ok(entries) = fs::read_dir(mods_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if name.eq_ignore_ascii_case(&target_file_name)
                    || name.ends_with(".part")
                {
                    continue;
                }
                let is_known = known_installed_files
                    .iter()
                    .any(|k| k.eq_ignore_ascii_case(name));
                if is_known || inspect_marlowww_jar_file(&path).is_some() {
                    if fs::remove_file(&path).is_ok() {
                        removed_old.push(name.to_string());
                    }
                }
            }
        }
    }

    let dest_path = mods_dir.join(&target_file_name);
    if dest_path.exists() {
        let _ = fs::remove_file(&dest_path);
    }
    fs::rename(&temp_path, &dest_path).map_err(|err| {
        let _ = fs::remove_file(&temp_path);
        format!("Failed to move JAR into mods folder: {err}")
    })?;

    Ok((target_file_name, in_game_name, dest_path, removed_old))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_marlowww_asset_patterns() {
        assert_eq!(
            parse_asset_metadata("marlowwwclient-5.5-1.21.11-android.jar"),
            (PlatformKind::Android, "1.21.11".to_string(), true)
        );
        assert_eq!(
            parse_asset_metadata("marlowwwclient-5.5-1.21.11.jar"),
            (PlatformKind::Desktop, "1.21.11".to_string(), true)
        );
        assert_eq!(
            parse_asset_metadata("marlowwwclient-5.5-26.3.jar"),
            (PlatformKind::Desktop, "26.3".to_string(), true)
        );
        assert_eq!(
            parse_asset_metadata("marlowwwclient-v5.jar"),
            (PlatformKind::Desktop, "Default".to_string(), false)
        );
    }

    #[test]
    fn filters_non_playable_jars() {
        assert!(!is_installable_jar("marlowwwclient-v5-sources.jar"));
        assert!(!is_installable_jar("marlowwwclient-v5-javadoc.jar"));
        assert!(is_installable_jar("marlowwwclient-v5.jar"));
        assert!(is_installable_jar("marlowwwclient-5.5-1.21.11-android.jar"));
    }

    #[test]
    fn rewrites_fabric_mod_json_display_name_inside_jar() {
        let tmp = std::env::temp_dir().join(format!(
            "marlowww_jar_rewrite_{}.jar",
            std::process::id()
        ));
        let _ = fs::remove_file(&tmp);

        {
            let f = File::create(&tmp).unwrap();
            let mut zw = ZipWriter::new(f);
            zw.start_file(
                "fabric.mod.json",
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
            zw.write_all(
                br#"{"schemaVersion":1,"id":"imnotcheatingyouare","version":"5.5","name":"Marlowww Client"}"#,
            )
            .unwrap();
            zw.start_file(
                "META-INF/jars/nested.jar",
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
            zw.write_all(&[1, 2, 3, 4, 5]).unwrap();
            zw.finish().unwrap();
        }

        rewrite_jar_display_name(&tmp, "Sodium Extra").unwrap();

        let inspected = inspect_marlowww_jar_file(&tmp).unwrap();
        assert_eq!(inspected.version.as_deref(), Some("5.5"));
        assert_eq!(inspected.display_name.as_deref(), Some("Sodium Extra"));
        assert_eq!(inspected.custom_ghost_name(), Some("Sodium Extra"));

        let mut verify = ZipArchive::new(File::open(&tmp).unwrap()).unwrap();
        let nested = verify.by_name("META-INF/jars/nested.jar").unwrap();
        assert_eq!(nested.compression(), CompressionMethod::Stored);
        drop(nested);
        drop(verify);

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn live_github_download_and_fabric_mod_json_rename_e2e() {
        if std::env::var_os("MARLOWWW_E2E_DOWNLOAD").is_none() {
            return;
        }
        let releases = fetch_releases().expect("fetch_releases failed");
        let latest = &releases[0];
        let asset = latest.assets[0].clone();

        let out_dir = std::env::temp_dir().join(format!(
            "marlowww_live_e2e_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&out_dir);
        fs::create_dir_all(&out_dir).unwrap();

        let (tx, rx) = async_channel::unbounded();
        download_and_install(
            asset,
            out_dir.clone(),
            Some("Custom Renamed Mod".to_string()),
            true,
            Vec::new(),
            tx,
        );

        let mut saw_progress = false;
        let mut final_dest = None;
        while let Ok(msg) = rx.recv_blocking() {
            match msg {
                DownloadUpdate::Progress { downloaded, .. } => {
                    if downloaded > 0 {
                        saw_progress = true;
                    }
                }
                DownloadUpdate::Finishing => {}
                DownloadUpdate::Done {
                    file_name,
                    in_game_name,
                    dest_path,
                    ..
                } => {
                    assert_eq!(file_name, "custom-renamed-mod.jar");
                    assert_eq!(in_game_name, "Custom Renamed Mod");
                    final_dest = Some(dest_path);
                    break;
                }
                DownloadUpdate::Failed(err) => panic!("Live download failed: {err}"),
            }
        }

        assert!(saw_progress, "Expected streaming progress events");
        let dest = final_dest.expect("Expected Done event");
        assert!(dest.is_file());

        let mut zf = ZipArchive::new(File::open(&dest).unwrap()).unwrap();
        let mut mod_json_str = String::new();
        zf.by_name("fabric.mod.json")
            .unwrap()
            .read_to_string(&mut mod_json_str)
            .unwrap();
        let parsed: Value = serde_json::from_str(&mod_json_str).unwrap();
        assert_eq!(
            parsed.get("name").and_then(|v| v.as_str()),
            Some("Custom Renamed Mod")
        );
        assert_eq!(
            parsed.get("id").and_then(|v| v.as_str()),
            Some("imnotcheatingyouare")
        );

        let mut main_cls = Vec::new();
        zf.by_name("com/eclipseware/imnotcheatingyouare/Imnotcheatingyouare.class")
            .unwrap()
            .read_to_end(&mut main_cls)
            .unwrap();
        assert!(main_cls.len() > 100);
        drop(zf);

        let _ = fs::remove_dir_all(&out_dir);
    }
}
