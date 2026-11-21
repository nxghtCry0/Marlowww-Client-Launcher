use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlatformKind {
    Desktop,
    Android,
    Linux,
    MacOS,
}

impl PlatformKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Desktop => "Desktop",
            Self::Android => "Android",
            Self::Linux => "Linux",
            Self::MacOS => "macOS",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAsset {
    pub name: String,
    pub download_url: String,
    pub size: u64,
    pub platform: PlatformKind,
    pub mc_version: String,
    pub is_explicit_mc_version: bool,
}

impl ReleaseAsset {
    pub fn formatted_size(&self) -> String {
        format_bytes(self.size)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub id: u64,
    pub tag_name: String,
    pub name: String,
    pub published_date: String,
    pub prerelease: bool,
    pub html_url: String,
    pub assets: Vec<ReleaseAsset>,
}

impl ReleaseInfo {
    pub fn display_title(&self) -> &str {
        if self.name.trim().is_empty() {
            &self.tag_name
        } else {
            &self.name
        }
    }

    pub fn normalized_version(&self) -> String {
        normalize_mod_version(&self.tag_name)
    }

    pub fn platforms(&self) -> Vec<PlatformKind> {
        let mut out: Vec<PlatformKind> = self.assets.iter().map(|a| a.platform).collect();
        out.sort();
        out.dedup();
        out
    }

    pub fn assets_for_platform(&self, platform: PlatformKind) -> Vec<&ReleaseAsset> {
        let mut list: Vec<&ReleaseAsset> = self
            .assets
            .iter()
            .filter(|a| a.platform == platform)
            .collect();
        list.sort_by(|a, b| {
            b.is_explicit_mc_version
                .cmp(&a.is_explicit_mc_version)
                .then_with(|| compare_versions_desc(&a.mc_version, &b.mc_version))
                .then_with(|| a.name.cmp(&b.name))
        });
        list
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LauncherKind {
    Official,
    Prism,
    Modrinth,
    OneClient,
    MultiMc,
    CurseForge,
    AtLauncher,
    Custom,
}

impl LauncherKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Official => "Official",
            Self::Prism => "Prism",
            Self::Modrinth => "Modrinth",
            Self::OneClient => "OneClient",
            Self::MultiMc => "MultiMC",
            Self::CurseForge => "CurseForge",
            Self::AtLauncher => "ATLauncher",
            Self::Custom => "Custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LauncherFilter {
    All,
    Prism,
    Modrinth,
    OneClient,
    Official,
    Other,
}

impl LauncherFilter {
    pub const ALL: [Self; 6] = [
        Self::All,
        Self::Prism,
        Self::Modrinth,
        Self::OneClient,
        Self::Official,
        Self::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Prism => "Prism",
            Self::Modrinth => "Modrinth",
            Self::OneClient => "OneClient",
            Self::Official => "Minecraft",
            Self::Other => "Other",
        }
    }

    pub fn matches(self, kind: LauncherKind) -> bool {
        match self {
            Self::All => true,
            Self::Prism => kind == LauncherKind::Prism,
            Self::Modrinth => kind == LauncherKind::Modrinth,
            Self::OneClient => kind == LauncherKind::OneClient,
            Self::Official => kind == LauncherKind::Official,
            Self::Other => matches!(
                kind,
                LauncherKind::MultiMc
                    | LauncherKind::CurseForge
                    | LauncherKind::AtLauncher
                    | LauncherKind::Custom
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FabricStatus {
    Detected {
        loader_version: Option<String>,
        mc_versions: Vec<String>,
        source: String,
    },
    Missing {
        reason: String,
    },
}

impl FabricStatus {
    pub fn is_detected(&self) -> bool {
        matches!(self, Self::Detected { .. })
    }

    pub fn mc_versions(&self) -> &[String] {
        match self {
            Self::Detected { mc_versions, .. } => mc_versions,
            Self::Missing { .. } => &[],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledModInfo {
    pub file_name: String,
    pub version: Option<String>,
    pub display_name: Option<String>,
    pub is_android: bool,
}

impl InstalledModInfo {
    pub fn custom_ghost_name(&self) -> Option<&str> {
        let name = self.display_name.as_deref()?.trim();
        if name.is_empty() {
            return None;
        }
        let lower = name.to_ascii_lowercase();
        if lower.contains("marlow") || lower == "imnotcheatingyouare" {
            None
        } else {
            Some(name)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceInfo {
    pub display_name: String,
    pub launcher: LauncherKind,
    pub selected_path: PathBuf,
    pub game_dir: PathBuf,
    pub mods_dir: PathBuf,
    pub fabric: FabricStatus,
    pub installed_mods: Vec<InstalledModInfo>,
}

impl InstanceInfo {
    pub fn has_installed_mod(&self) -> bool {
        !self.installed_mods.is_empty()
    }

    pub fn primary_installed_mod(&self) -> Option<&InstalledModInfo> {
        self.installed_mods.first()
    }

    pub fn matches_search(&self, query: &str) -> bool {
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return true;
        }
        if self.display_name.to_ascii_lowercase().contains(&q) {
            return true;
        }
        if self.launcher.label().to_ascii_lowercase().contains(&q) {
            return true;
        }
        if self
            .fabric
            .mc_versions()
            .iter()
            .any(|v| v.to_ascii_lowercase().contains(&q))
        {
            return true;
        }
        if let Some(folder) = self.selected_path.file_name().and_then(|n| n.to_str()) {
            if folder.to_ascii_lowercase().contains(&q) {
                return true;
            }
        }
        self.installed_mods.iter().any(|m| {
            m.file_name.to_ascii_lowercase().contains(&q)
                || m.display_name
                    .as_deref()
                    .is_some_and(|d| d.to_ascii_lowercase().contains(&q))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WizardStep {
    SelectVersion,
    NameJar,
    SelectInstance,
    Install,
}

impl WizardStep {
    pub const ALL: [Self; 4] = [
        Self::SelectVersion,
        Self::NameJar,
        Self::SelectInstance,
        Self::Install,
    ];

    pub fn number(self) -> usize {
        match self {
            Self::SelectVersion => 1,
            Self::NameJar => 2,
            Self::SelectInstance => 3,
            Self::Install => 4,
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::SelectVersion => "Versions",
            Self::NameJar => "Renaming",
            Self::SelectInstance => "Instances",
            Self::Install => "Install",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpotlightZone {
    None,
    LeftCanvas,
    RightInspector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TourStep {
    Hidden,
    Welcome,
    WhatIsThis,
    FabricInstall,
    SpotVersion,
    SpotNameJar,
    SpotInstance,
    SpotInstall,
    SpotComplete,
}

impl TourStep {
    pub fn is_modal_intro(self) -> bool {
        matches!(self, Self::Welcome | Self::WhatIsThis | Self::FabricInstall)
    }

    pub fn is_spotlight(self) -> bool {
        matches!(
            self,
            Self::SpotVersion
                | Self::SpotNameJar
                | Self::SpotInstance
                | Self::SpotInstall
                | Self::SpotComplete
        )
    }

    pub fn spotlight_zone(self) -> SpotlightZone {
        match self {
            Self::SpotVersion | Self::SpotNameJar | Self::SpotInstance => SpotlightZone::LeftCanvas,
            Self::SpotInstall | Self::SpotComplete => SpotlightZone::RightInspector,
            _ => SpotlightZone::None,
        }
    }

    pub fn target_wizard_step(self) -> Option<WizardStep> {
        match self {
            Self::SpotVersion => Some(WizardStep::SelectVersion),
            Self::SpotNameJar => Some(WizardStep::NameJar),
            Self::SpotInstance => Some(WizardStep::SelectInstance),
            Self::SpotInstall | Self::SpotComplete => Some(WizardStep::Install),
            _ => None,
        }
    }

    pub fn step_index(self) -> usize {
        match self {
            Self::Hidden => 0,
            Self::Welcome => 1,
            Self::WhatIsThis => 2,
            Self::FabricInstall => 3,
            Self::SpotVersion => 4,
            Self::SpotNameJar => 5,
            Self::SpotInstance => 6,
            Self::SpotInstall => 7,
            Self::SpotComplete => 8,
        }
    }

    pub const TOTAL_STEPS: usize = 8;

    pub fn next(self) -> Self {
        match self {
            Self::Hidden => Self::Hidden,
            Self::Welcome => Self::WhatIsThis,
            Self::WhatIsThis => Self::FabricInstall,
            Self::FabricInstall => Self::SpotVersion,
            Self::SpotVersion => Self::SpotNameJar,
            Self::SpotNameJar => Self::SpotInstance,
            Self::SpotInstance => Self::SpotInstall,
            Self::SpotInstall => Self::SpotComplete,
            Self::SpotComplete => Self::Hidden,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Hidden | Self::Welcome => Self::Welcome,
            Self::WhatIsThis => Self::Welcome,
            Self::FabricInstall => Self::WhatIsThis,
            Self::SpotVersion => Self::FabricInstall,
            Self::SpotNameJar => Self::SpotVersion,
            Self::SpotInstance => Self::SpotNameJar,
            Self::SpotInstall => Self::SpotInstance,
            Self::SpotComplete => Self::SpotInstall,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Hidden => "",
            Self::Welcome => "Welcome To Marlowww Client's Launcher!",
            Self::WhatIsThis => "What is This?",
            Self::FabricInstall => "Fabric Installation",
            Self::SpotVersion => "Step 1: Selecting Your Version",
            Self::SpotNameJar => "Step 2: Renaming",
            Self::SpotInstance => "Step 3: Selecting Your Instance",
            Self::SpotInstall => "Step 4: Installing",
            Self::SpotComplete => "Complete & Ready to Launch",
        }
    }

    pub fn body(self) -> &'static str {
        match self {
            Self::Hidden => "",
            Self::Welcome => {
                "We've listened to your feedback and made the entire setup process clean, linear, and fast so you can get straight into downloading the client."
            }
            Self::WhatIsThis => {
                "Instead of launching Minecraft directly, this launcher sets up the mod inside your instance folder for you and configures everything automatically. All you need is Fabric."
            }
            Self::FabricInstall => {
                "Using the normal Minecraft Launcher? Download Fabric Loader below, select Windows, open the installer, and click Install to add the Fabric profile. Using Prism, Modrinth, or OneClient? Fabric is handled per instance, so you can pick yours right away."
            }
            Self::SpotVersion => {
                "Pick your preferred Marlowww Client release, platform, and Minecraft version (from 1.21.11 up to the latest release). Select the Android build if your device runs mobile or an alternate processor."
            }
            Self::SpotNameJar => {
                "Pick your own custom name before choosing your instance. This updates both the JAR file and fabric.mod.json so your chosen name appears in game instead of 'Marlowww Client'."
            }
            Self::SpotInstance => {
                "Search or filter all your detected instances across Prism, Modrinth, OneClient, and the official Minecraft launcher. Look for your Minecraft version on official profiles, or search the custom name you gave your instance."
            }
            Self::SpotInstall => {
                "You're on the last and easiest step. Review your setup, hit the Install button, or use the one-click Update button anytime to swap in the newest build for your version."
            }
            Self::SpotComplete => {
                "Once installation finishes, launch Minecraft from your usual launcher and use Marlowww Client as normal. You can replay this guide anytime from the Guide button in the top bar."
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleasesState {
    Loading,
    Loaded(Vec<ReleaseInfo>),
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstallState {
    Idle,
    Downloading {
        file_name: String,
        downloaded: u64,
        total: u64,
        speed_bps: f64,
    },
    Finishing {
        file_name: String,
    },
    Success {
        file_name: String,
        in_game_name: String,
        dest_path: PathBuf,
        removed_old: Vec<String>,
    },
    Error(String),
}

impl InstallState {
    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Downloading { .. } | Self::Finishing { .. })
    }
}

#[derive(Debug, Clone)]
pub enum DownloadUpdate {
    Progress {
        downloaded: u64,
        total: u64,
        speed_bps: f64,
    },
    Finishing,
    Done {
        file_name: String,
        in_game_name: String,
        dest_path: PathBuf,
        removed_old: Vec<String>,
    },
    Failed(String),
}

pub fn resolve_latest_update_for_instance<'a>(
    instance: &InstanceInfo,
    releases: &'a [ReleaseInfo],
) -> Option<(usize, &'a ReleaseInfo, &'a ReleaseAsset)> {
    if releases.is_empty() {
        return None;
    }

    let prefer_android = instance
        .primary_installed_mod()
        .is_some_and(|m| m.is_android);
    let preferred_platform = if prefer_android {
        PlatformKind::Android
    } else {
        PlatformKind::Desktop
    };
    let mc_versions = instance.fabric.mc_versions();

    if !mc_versions.is_empty() {
        for (idx, rel) in releases.iter().enumerate() {
            let plat_assets = rel.assets_for_platform(preferred_platform);
            if let Some(asset) = plat_assets.iter().copied().find(|a| {
                a.is_explicit_mc_version && mc_versions.iter().any(|m| m == &a.mc_version)
            }) {
                return Some((idx, rel, asset));
            }
            if preferred_platform != PlatformKind::Desktop {
                let desk_assets = rel.assets_for_platform(PlatformKind::Desktop);
                if let Some(asset) = desk_assets.iter().copied().find(|a| {
                    a.is_explicit_mc_version && mc_versions.iter().any(|m| m == &a.mc_version)
                }) {
                    return Some((idx, rel, asset));
                }
            }
        }
    }

    let first = &releases[0];
    let fallback = first
        .assets_for_platform(preferred_platform)
        .into_iter()
        .next()
        .or_else(|| {
            first
                .assets_for_platform(PlatformKind::Desktop)
                .into_iter()
                .next()
        })
        .or_else(|| first.assets.first())?;

    Some((0, first, fallback))
}

pub fn is_instance_outdated(instance: &InstanceInfo, releases: &[ReleaseInfo]) -> bool {
    let Some(installed) = instance.primary_installed_mod() else {
        return false;
    };
    let Some((_, target_rel, target_asset)) =
        resolve_latest_update_for_instance(instance, releases)
    else {
        return false;
    };
    if let Some(inst_ver) = &installed.version {
        let norm_inst = normalize_mod_version(inst_ver);
        let norm_rel = target_rel.normalized_version();
        if !norm_inst.is_empty() && !norm_rel.is_empty() {
            return norm_inst != norm_rel;
        }
    }
    installed.file_name != target_asset.name
}

pub fn normalize_mod_version(raw: &str) -> String {
    let trimmed = raw
        .trim()
        .strip_prefix('v')
        .or_else(|| raw.trim().strip_prefix('V'))
        .unwrap_or(raw.trim());
    trimmed
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>()
        .trim_end_matches('.')
        .to_string()
}

pub fn sanitize_jar_filename(custom_name: &str, default_asset_name: &str) -> String {
    let trimmed = custom_name.trim();
    if trimmed.is_empty() {
        return default_asset_name.to_string();
    }
    let explicit_ext = trimmed.ends_with(".jar") || trimmed.ends_with(".JAR");
    let without_ext = trimmed
        .strip_suffix(".jar")
        .or_else(|| trimmed.strip_suffix(".JAR"))
        .unwrap_or(trimmed)
        .trim();

    let mut cleaned = String::with_capacity(without_ext.len());
    let mut prev_dash = false;
    for ch in without_ext.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
            cleaned.push(if explicit_ext {
                ch
            } else {
                ch.to_ascii_lowercase()
            });
            prev_dash = false;
        } else if ch.is_whitespace() && !prev_dash && !cleaned.is_empty() {
            cleaned.push('-');
            prev_dash = true;
        }
    }
    let cleaned = cleaned.trim_matches(|c| c == '-' || c == '.').to_string();
    if cleaned.is_empty() {
        default_asset_name.to_string()
    } else {
        format!("{cleaned}.jar")
    }
}

pub fn effective_in_game_name(custom_name: &str) -> String {
    let trimmed = custom_name.trim();
    if trimmed.is_empty() {
        "Marlowww Client".to_string()
    } else {
        trimmed
            .strip_suffix(".jar")
            .or_else(|| trimmed.strip_suffix(".JAR"))
            .unwrap_or(trimmed)
            .trim()
            .to_string()
    }
}

pub fn version_codename(mc_version: &str) -> &'static str {
    if mc_version.starts_with("26.3") {
        "Wilderness Bound"
    } else if mc_version.starts_with("26.2") {
        "Chaos Cubed"
    } else if mc_version.starts_with("26.1") {
        "Tiny Takeover"
    } else if mc_version.starts_with("1.21.11") {
        "Mounts & Mayhem"
    } else if mc_version.starts_with("1.21") {
        "Tricky Trials"
    } else {
        "Universal Edition"
    }
}

pub fn version_summary_blurb(mc_version: &str) -> &'static str {
    if mc_version.starts_with("26.3") {
        "Latest 26.3 Fabric build tuned for modern combat, crystal timing, and full module compatibility."
    } else if mc_version.starts_with("26.2") {
        "Targeted 26.2 Fabric build with responsive movement hooks and clean HUD rendering."
    } else if mc_version.starts_with("26.1") {
        "Minecraft 26.1.x Tiny Takeover release build verified against Fabric Loader profiles."
    } else if mc_version.starts_with("1.21") {
        "Flagship 1.21.11 Fabric client build supporting both Desktop Java and Android ARM runtimes."
    } else {
        "Standalone Fabric release archive ready to drop straight into your instance mods folder."
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

pub fn compare_versions_desc(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> Vec<u32> {
        s.split('.')
            .map(|part| {
                part.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<u32>()
                    .unwrap_or(0)
            })
            .collect()
    };
    let va = parse(a);
    let vb = parse(b);
    let max_len = va.len().max(vb.len());
    for i in 0..max_len {
        let na = *va.get(i).unwrap_or(&0);
        let nb = *vb.get(i).unwrap_or(&0);
        if na != nb {
            return nb.cmp(&na);
        }
    }
    b.cmp(a)
}
