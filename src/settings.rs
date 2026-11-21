use std::{
    env,
    fs,
    path::PathBuf,
};

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct LauncherSettings {
    pub onboarding_completed: bool,
    pub replace_older_builds: bool,
    pub custom_jar_name: String,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            onboarding_completed: false,
            replace_older_builds: true,
            custom_jar_name: String::new(),
        }
    }
}

pub fn settings_path() -> PathBuf {
    if let Some(appdata) = env::var_os("APPDATA").map(PathBuf::from) {
        appdata.join("MarlowwwLauncher").join("settings.json")
    } else if let Some(home) = env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
    {
        home.join(".marlowww-launcher").join("settings.json")
    } else {
        PathBuf::from("marlowww-launcher-settings.json")
    }
}

pub fn load_settings() -> LauncherSettings {
    let path = settings_path();
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<LauncherSettings>(&raw).ok())
        .unwrap_or_default()
}

pub fn save_settings(settings: &LauncherSettings) {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(serialized) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(path, serialized);
    }
}
