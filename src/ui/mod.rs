pub mod components;
pub mod onboarding;

use std::path::PathBuf;

use freya::prelude::*;

use crate::{
    github::{download_and_install, fetch_releases},
    minecraft::{discover_instances, inspect_instance},
    models::{
        DownloadUpdate, FabricStatus, InstallState, InstanceInfo, LauncherFilter, PlatformKind,
        ReleaseAsset, ReleaseInfo, ReleasesState, SpotlightZone, TourStep, WizardStep,
        effective_in_game_name, format_bytes, is_instance_outdated,
        resolve_latest_update_for_instance, sanitize_jar_filename, version_codename,
        version_summary_blurb,
    },
    settings::{LauncherSettings, load_settings, save_settings},
    theme,
    ui::{
        components::{
            ActionButton, ArtworkCard, BrandLogo, ButtonVariant, DisguisePresetCard, DropBackdrop,
            IconSquareButton, MotionTransition, PhosphorIcon, RefreshButton, SegmentPill,
            SkeletonCardGrid, SlimProgressBar, ToggleSwitch, TopNavTab, UnderlineTab,
        },
        onboarding::{OnboardingModalOverlay, SpotlightCoachBar, SpotlightZoneWrapper},
    },
};

const RENAME_PRESETS: &[(&str, &str)] = &[
    ("Marlowww Client (Default)", ""),
    ("Sodium Extra", "Sodium Extra"),
    ("FerriteCore", "FerriteCore"),
    ("EntityCulling", "EntityCulling"),
    ("Dynamic FPS", "Dynamic FPS"),
    ("ImmediatelyFast", "ImmediatelyFast"),
];

fn sync_selection_for_release(
    release: &ReleaseInfo,
    instance: Option<&InstanceInfo>,
    mut selected_platform: State<PlatformKind>,
    mut selected_asset_name: State<Option<String>>,
) {
    let platforms = release.platforms();
    let current_platform = *selected_platform.peek();
    let next_platform = if platforms.contains(&current_platform) {
        current_platform
    } else {
        platforms.first().copied().unwrap_or(PlatformKind::Desktop)
    };
    selected_platform.set_if_modified(next_platform);

    let assets = release.assets_for_platform(next_platform);
    let preferred_mc = instance.map(|i| i.fabric.mc_versions()).unwrap_or(&[]);

    let matched = assets
        .iter()
        .find(|a| a.is_explicit_mc_version && preferred_mc.iter().any(|m| m == &a.mc_version))
        .or_else(|| assets.first());

    selected_asset_name.set(matched.map(|a| a.name.clone()));
}

fn init_launcher_theme() {
    use_init_theme(|| {
        let mut t = dark_theme();
        t.colors.background = theme::BG_CANVAS;
        t.colors.surface_primary = theme::BORDER_HOVER;
        t.colors.surface_secondary = theme::BG_HOVER;
        t.colors.surface_tertiary = theme::BG_CONTROL;
        t.colors.surface_inverse = Color::from_rgb(56, 64, 78);
        t.colors.surface_inverse_secondary = Color::from_rgb(76, 86, 104);
        t.colors.surface_inverse_tertiary = Color::from_rgb(98, 110, 132);
        t.colors.border = theme::BORDER_CONTROL;
        t.colors.primary = theme::ACCENT;
        t.colors.text_primary = theme::TEXT_PRIMARY;
        t.colors.text_secondary = theme::TEXT_SECONDARY;
        t
    });
}

fn custom_input_colors() -> InputColorsThemePartial {
    InputColorsThemePartial {
        background: Some(Preference::Specific(theme::BG_CONTROL)),
        focus_background: Some(Preference::Specific(theme::BG_CARD)),
        border_fill: Some(Preference::Specific(theme::BORDER_CONTROL)),
        focus_border_fill: Some(Preference::Specific(theme::ACCENT_RING)),
        color: Some(Preference::Specific(theme::TEXT_PRIMARY)),
        placeholder_color: Some(Preference::Specific(theme::TEXT_MUTED)),
    }
}

pub fn app() -> impl IntoElement {
    let initial_settings = use_hook(load_settings);
    let mut releases_state = use_state(|| ReleasesState::Loading);
    let mut detected_instances = use_state(Vec::<InstanceInfo>::new);
    let selected_instance = use_state(|| None::<InstanceInfo>);
    let selected_release_idx = use_state(|| 0usize);
    let selected_platform = use_state(|| PlatformKind::Desktop);
    let selected_asset_name = use_state(|| None::<String>);
    let replace_existing = use_state(|| initial_settings.replace_older_builds);
    let custom_jar_name = use_state(|| initial_settings.custom_jar_name.clone());
    let wizard_step = use_state(|| WizardStep::SelectVersion);
    let tour_step = use_state(|| {
        if initial_settings.onboarding_completed {
            TourStep::Hidden
        } else {
            TourStep::Welcome
        }
    });
    let instance_search = use_state(String::new);
    let launcher_filter = use_state(|| LauncherFilter::All);
    let install_state = use_state(|| InstallState::Idle);

    use_hook(move || {
        spawn_in_window(async move {
            let discovered = thread(discover_instances).await;
            detected_instances.set(discovered);
        });

        spawn_in_window(async move {
            match thread(fetch_releases).await {
                Ok(list) => {
                    if let Some(first) = list.first() {
                        let inst = selected_instance.peek().clone();
                        sync_selection_for_release(
                            first,
                            inst.as_ref(),
                            selected_platform,
                            selected_asset_name,
                        );
                    }
                    releases_state.set(ReleasesState::Loaded(list));
                }
                Err(err) => {
                    releases_state.set(ReleasesState::Error(err));
                }
            }
        });
    });

    LauncherShell {
        releases_state,
        detected_instances,
        selected_instance,
        selected_release_idx,
        selected_platform,
        selected_asset_name,
        replace_existing,
        custom_jar_name,
        wizard_step,
        tour_step,
        instance_search,
        launcher_filter,
        install_state,
        persist_settings: true,
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct LauncherShell {
    pub releases_state: State<ReleasesState>,
    pub detected_instances: State<Vec<InstanceInfo>>,
    pub selected_instance: State<Option<InstanceInfo>>,
    pub selected_release_idx: State<usize>,
    pub selected_platform: State<PlatformKind>,
    pub selected_asset_name: State<Option<String>>,
    pub replace_existing: State<bool>,
    pub custom_jar_name: State<String>,
    pub wizard_step: State<WizardStep>,
    pub tour_step: State<TourStep>,
    pub instance_search: State<String>,
    pub launcher_filter: State<LauncherFilter>,
    pub install_state: State<InstallState>,
    pub persist_settings: bool,
}

impl LauncherShell {
    fn save_prefs(self, onboarding_done: bool) {
        if !self.persist_settings {
            return;
        }
        let replace_older_builds = { *self.replace_existing.peek() };
        let custom_jar_name = { self.custom_jar_name.peek().clone() };
        let prefs = LauncherSettings {
            onboarding_completed: onboarding_done,
            replace_older_builds,
            custom_jar_name,
        };
        spawn_in_window(async move {
            thread(move || save_settings(&prefs)).await;
        });
    }

    fn trigger_refresh(mut self) {
        self.releases_state.set(ReleasesState::Loading);
        let mut detected_instances = self.detected_instances;
        spawn_in_window(async move {
            let discovered = thread(discover_instances).await;
            detected_instances.set(discovered);
        });
        let mut releases_state = self.releases_state;
        let mut selected_release_idx = self.selected_release_idx;
        let selected_instance = self.selected_instance;
        let selected_platform = self.selected_platform;
        let selected_asset_name = self.selected_asset_name;
        spawn_in_window(async move {
            match thread(fetch_releases).await {
                Ok(list) => {
                    selected_release_idx.set(0);
                    if let Some(first) = list.first() {
                        let inst = { selected_instance.peek().clone() };
                        sync_selection_for_release(
                            first,
                            inst.as_ref(),
                            selected_platform,
                            selected_asset_name,
                        );
                    }
                    releases_state.set(ReleasesState::Loaded(list));
                }
                Err(err) => {
                    releases_state.set(ReleasesState::Error(err));
                }
            }
        });
    }

    fn apply_instance(self, path: PathBuf) {
        let mut selected_instance = self.selected_instance;
        let mut install_state = self.install_state;
        let releases_state = self.releases_state;
        let selected_release_idx = self.selected_release_idx;
        let selected_platform = self.selected_platform;
        let mut selected_asset_name = self.selected_asset_name;
        spawn_in_window(async move {
            let inspected = thread(move || inspect_instance(path)).await;
            let mc_versions = inspected.fabric.mc_versions().to_vec();
            selected_instance.set(Some(inspected));
            install_state.set(InstallState::Idle);

            let matched_name = {
                let rel_guard = releases_state.peek();
                let idx = *selected_release_idx.peek();
                let plat = *selected_platform.peek();
                if let ReleasesState::Loaded(list) = &*rel_guard {
                    list.get(idx).and_then(|rel| {
                        rel.assets_for_platform(plat)
                            .into_iter()
                            .find(|a| {
                                a.is_explicit_mc_version
                                    && mc_versions.iter().any(|m| m == &a.mc_version)
                            })
                            .map(|a| a.name.clone())
                    })
                } else {
                    None
                }
            };
            if let Some(name) = matched_name {
                selected_asset_name.set(Some(name));
            }
        });
    }

    fn resolve_current_asset(self) -> Option<ReleaseAsset> {
        let rel_guard = self.releases_state.peek();
        let ReleasesState::Loaded(list) = &*rel_guard else {
            return None;
        };
        let idx = *self.selected_release_idx.peek();
        let rel = list.get(idx)?;
        let plat = *self.selected_platform.peek();
        let for_plat = rel.assets_for_platform(plat);
        let cur_name = self.selected_asset_name.peek().clone();
        cur_name
            .as_ref()
            .and_then(|n| for_plat.iter().find(|a| &a.name == n).copied())
            .or_else(|| for_plat.first().copied())
            .cloned()
    }

    fn install_current_selection(self) {
        let Some(asset) = self.resolve_current_asset() else {
            return;
        };
        let inst_opt = { self.selected_instance.peek().clone() };
        let Some(inst) = inst_opt else {
            return;
        };
        self.start_install_job(asset, inst, None, false);
    }

    fn start_install_job(
        mut self,
        asset: ReleaseAsset,
        inst: InstanceInfo,
        name_override: Option<String>,
        force_replace: bool,
    ) {
        let is_busy = { self.install_state.peek().is_busy() };
        if !inst.fabric.is_detected() || is_busy {
            return;
        }
        let replace_pref = { *self.replace_existing.peek() };
        let replace = force_replace || replace_pref;
        let chosen_name =
            name_override.unwrap_or_else(|| self.custom_jar_name.peek().clone());
        let out_file_name = sanitize_jar_filename(&chosen_name, &asset.name);
        let mods_dir = inst.mods_dir.clone();
        let selected_path = inst.selected_path.clone();
        let existing_files: Vec<String> = inst
            .installed_mods
            .iter()
            .map(|m| m.file_name.clone())
            .collect();

        self.wizard_step.set(WizardStep::Install);
        self.install_state.set(InstallState::Downloading {
            file_name: out_file_name.clone(),
            downloaded: 0,
            total: asset.size,
            speed_bps: 0.0,
        });
        self.save_prefs(true);

        let custom_opt = if chosen_name.trim().is_empty() {
            None
        } else {
            Some(chosen_name)
        };

        let (tx, rx) = async_channel::unbounded();
        std::thread::spawn(move || {
            download_and_install(asset, mods_dir, custom_opt, replace, existing_files, tx);
        });

        let mut install_state = self.install_state;
        let mut detected_instances = self.detected_instances;
        let mut selected_instance = self.selected_instance;
        spawn_in_window(async move {
            while let Ok(update) = rx.recv().await {
                match update {
                    DownloadUpdate::Progress {
                        downloaded,
                        total,
                        speed_bps,
                    } => {
                        install_state.set(InstallState::Downloading {
                            file_name: out_file_name.clone(),
                            downloaded,
                            total,
                            speed_bps,
                        });
                    }
                    DownloadUpdate::Finishing => {
                        install_state.set(InstallState::Finishing {
                            file_name: out_file_name.clone(),
                        });
                    }
                    DownloadUpdate::Done {
                        file_name,
                        in_game_name,
                        dest_path,
                        removed_old,
                    } => {
                        let path_for_inspect = selected_path.clone();
                        let refreshed = thread(move || inspect_instance(path_for_inspect)).await;
                        let mut cur_list = { detected_instances.peek().clone() };
                        if let Some(slot) = cur_list
                            .iter_mut()
                            .find(|i| i.game_dir == refreshed.game_dir)
                        {
                            *slot = refreshed.clone();
                        }
                        detected_instances.set(cur_list);
                        selected_instance.set(Some(refreshed));
                        install_state.set(InstallState::Success {
                            file_name,
                            in_game_name,
                            dest_path,
                            removed_old,
                        });
                        break;
                    }
                    DownloadUpdate::Failed(err) => {
                        install_state.set(InstallState::Error(err));
                        break;
                    }
                }
            }
        });
    }

    fn trigger_quick_update(mut self, inst: InstanceInfo) {
        let releases_snapshot = { self.releases_state.peek().clone() };
        let ReleasesState::Loaded(list) = releases_snapshot else {
            return;
        };
        let Some((rel_idx, _rel, asset)) = resolve_latest_update_for_instance(&inst, &list) else {
            return;
        };
        let asset_cloned = asset.clone();
        self.selected_release_idx.set(rel_idx);
        self.selected_platform.set(asset_cloned.platform);
        self.selected_asset_name
            .set(Some(asset_cloned.name.clone()));
        self.selected_instance.set(Some(inst.clone()));

        let current_custom = { self.custom_jar_name.peek().clone() };
        let preserved_name = if current_custom.trim().is_empty() {
            inst.primary_installed_mod()
                .and_then(|m| m.custom_ghost_name())
                .map(String::from)
        } else {
            Some(current_custom)
        };

        self.start_install_job(asset_cloned, inst, preserved_name, true);
    }
}

impl Component for LauncherShell {
    fn render(&self) -> impl IntoElement {
        init_launcher_theme();

        let shell = *self;
        let releases_state = self.releases_state;
        let detected_instances = self.detected_instances;
        let mut selected_instance = self.selected_instance;
        let mut selected_release_idx = self.selected_release_idx;
        let mut selected_platform = self.selected_platform;
        let mut selected_asset_name = self.selected_asset_name;
        let mut replace_existing = self.replace_existing;
        let mut custom_jar_name = self.custom_jar_name;
        let mut wizard_step = self.wizard_step;
        let mut tour_step = self.tour_step;
        let instance_search = self.instance_search;
        let mut launcher_filter = self.launcher_filter;
        let mut install_state = self.install_state;

        let mut is_dragging_file = use_state(|| false);

        let releases_snapshot = releases_state.read().clone();
        let instance_snapshot = selected_instance.read().clone();
        let install_snapshot = install_state.read().clone();
        let detected_snapshot = detected_instances.read().clone();
        let cur_release_idx = selected_release_idx();
        let cur_platform = selected_platform();
        let cur_asset_name = selected_asset_name.read().clone();
        let cur_custom_name = custom_jar_name.read().clone();
        let cur_step = wizard_step();
        let cur_tour = tour_step();
        let cur_search = instance_search.read().clone();
        let cur_filter = launcher_filter();

        let loaded_releases: Vec<ReleaseInfo> = match &releases_snapshot {
            ReleasesState::Loaded(list) => list.clone(),
            _ => Vec::new(),
        };

        let active_release: Option<ReleaseInfo> = loaded_releases.get(cur_release_idx).cloned();

        let active_asset: Option<ReleaseAsset> = active_release.as_ref().and_then(|rel| {
            let for_plat = rel.assets_for_platform(cur_platform);
            cur_asset_name
                .as_ref()
                .and_then(|name| for_plat.iter().find(|a| &a.name == name).copied())
                .or_else(|| for_plat.first().copied())
                .cloned()
        });

        let fabric_ready = instance_snapshot
            .as_ref()
            .is_some_and(|i| i.fabric.is_detected());
        let can_install = fabric_ready && active_asset.is_some() && !install_snapshot.is_busy();

        let spot_zone = cur_tour.spotlight_zone();
        let spotlight_active = cur_tour.is_spotlight();
        let topbar_opacity = if spotlight_active { 0.35 } else { 1.0 };

        let active_mc_ver = instance_snapshot
            .as_ref()
            .and_then(|i| {
                if cur_step >= WizardStep::SelectInstance {
                    i.fabric.mc_versions().first().map(|s| s.as_str())
                } else {
                    None
                }
            })
            .or_else(|| active_asset.as_ref().map(|a| a.mc_version.as_str()))
            .unwrap_or("26.3");
        let (hero_banner_id, hero_banner_bytes) = theme::banner_for_version(active_mc_ver);

        let top_bar = rect()
            .width(Size::fill())
            .opacity(topbar_opacity)
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .main_align(Alignment::SpaceBetween)
            .padding(Gaps::new(12.0, 22.0, 10.0, 24.0))
            .background(theme::BG_TOPBAR)
            .child(BrandLogo::new(148.0, 34.8))
            .child(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(4.0)
                    .children(WizardStep::ALL.iter().map(|&step| {
                        let completed = match step {
                            WizardStep::SelectVersion => {
                                active_asset.is_some() && cur_step > WizardStep::SelectVersion
                            }
                            WizardStep::NameJar => {
                                cur_step > WizardStep::NameJar || !cur_custom_name.trim().is_empty()
                            }
                            WizardStep::SelectInstance => fabric_ready,
                            WizardStep::Install => {
                                matches!(install_snapshot, InstallState::Success { .. })
                            }
                        };
                        TopNavTab {
                            step,
                            active: cur_step == step,
                            completed,
                            on_select: NoArgCallback::new(move || {
                                wizard_step.set(step);
                            }),
                            key: DiffKey::None,
                        }
                        .key(step.number())
                    })),
            )
            .child(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(8.0)
                    .child(
                        ActionButton::new("Guide", move || {
                            wizard_step.set(WizardStep::SelectVersion);
                            tour_step.set(TourStep::Welcome);
                        })
                        .icon(theme::PHOSPHOR_COMPASS)
                        .variant(ButtonVariant::Subtle)
                        .compact(true),
                    )
                    .child(RefreshButton {
                        spinning: matches!(releases_snapshot, ReleasesState::Loading),
                        on_press: NoArgCallback::new(move || shell.trigger_refresh()),
                    }),
            );

        let left_canvas_body: Element = match cur_step {
            WizardStep::SelectVersion => {
                let rel_url = active_release
                    .as_ref()
                    .map(|r| r.html_url.clone())
                    .unwrap_or_default();

                let header_row = page_header_row(
                    "Versions",
                    "Pick a Marlowww Client release, platform, and target Minecraft version.",
                    (!rel_url.is_empty()).then(|| {
                        ActionButton::new("Release notes", move || {
                            let _ = open::that(&rel_url);
                        })
                        .trailing_icon(theme::PHOSPHOR_EXTERNAL)
                        .variant(ButtonVariant::Control)
                    }),
                );

                let content_area: Element = match &releases_snapshot {
                    ReleasesState::Loading => SkeletonCardGrid.into_element(),
                    ReleasesState::Error(err) => rect()
                        .width(Size::fill())
                        .vertical()
                        .spacing(12.0)
                        .padding(Gaps::new_all(20.0))
                        .corner_radius(theme::RADIUS_CARD)
                        .background(theme::BG_PANEL)
                        .border(Border::new().width(1.0).fill(theme::ERR_BORDER))
                        .child(
                            label()
                                .text("Unable to reach GitHub Releases")
                                .color(theme::TEXT_PRIMARY)
                                .font_size(14.5)
                                .font_weight(FontWeight::SEMI_BOLD),
                        )
                        .child(
                            label()
                                .text(err.clone())
                                .color(theme::TEXT_SECONDARY)
                                .font_size(12.5),
                        )
                        .child(
                            ActionButton::new("Retry", move || shell.trigger_refresh())
                                .variant(ButtonVariant::Primary),
                        )
                        .into_element(),
                    ReleasesState::Loaded(list) => {
                        let top_tabs_count = list.len().min(5);
                        let rel = active_release.clone().unwrap_or_else(|| list[0].clone());
                        let platforms = rel.platforms();
                        let matching_assets = rel.assets_for_platform(cur_platform);
                        let detected_mcs: Vec<String> = instance_snapshot
                            .as_ref()
                            .map(|i| i.fabric.mc_versions().to_vec())
                            .unwrap_or_default();
                        let anim_group = format!("ver-{}-{:?}", rel.id, cur_platform);
                        let rel_for_plat = rel.clone();
                        let row_count = matching_assets.len().div_ceil(3);

                        rect()
                            .width(Size::fill())
                            .height(Size::flex(1.0))
                            .vertical()
                            .content(Content::Flex)
                            .spacing(14.0)
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .horizontal()
                                    .content(Content::Flex)
                                    .main_align(Alignment::SpaceBetween)
                                    .cross_align(Alignment::End)
                                    .border(
                                        Border::new()
                                            .width(BorderWidth {
                                                top: 0.0,
                                                right: 0.0,
                                                bottom: 1.0,
                                                left: 0.0,
                                            })
                                            .fill(theme::DIVIDER),
                                    )
                                    .child(
                                        rect()
                                            .horizontal()
                                            .spacing(18.0)
                                            .children(list.iter().take(top_tabs_count).enumerate().map(
                                                |(idx, item)| {
                                                    let item_clone = item.clone();
                                                    UnderlineTab {
                                                        label: item.tag_name.clone(),
                                                        selected: idx == cur_release_idx,
                                                        on_press: NoArgCallback::new(move || {
                                                            selected_release_idx.set(idx);
                                                            install_state.set(InstallState::Idle);
                                                            let inst =
                                                                selected_instance.peek().clone();
                                                            sync_selection_for_release(
                                                                &item_clone,
                                                                inst.as_ref(),
                                                                selected_platform,
                                                                selected_asset_name,
                                                            );
                                                        }),
                                                        key: DiffKey::None,
                                                    }
                                                    .key(item.id)
                                                },
                                            )),
                                    )
                                    .maybe_child((platforms.len() > 1).then(|| {
                                        rect()
                                            .margin(Gaps::new(0.0, 0.0, 6.0, 0.0))
                                            .horizontal()
                                            .spacing(2.0)
                                            .padding(Gaps::new_all(3.0))
                                            .corner_radius(theme::RADIUS_CONTROL)
                                            .background(theme::BG_CONTROL)
                                            .border(
                                                Border::new()
                                                    .width(1.0)
                                                    .fill(theme::BORDER_CONTROL),
                                            )
                                            .children(platforms.iter().map(|&plat| {
                                                let rel_copy = rel_for_plat.clone();
                                                SegmentPill {
                                                    label: plat.label().to_string(),
                                                    selected: plat == cur_platform,
                                                    on_press: NoArgCallback::new(move || {
                                                        selected_platform.set(plat);
                                                        install_state.set(InstallState::Idle);
                                                        let inst =
                                                            selected_instance.peek().clone();
                                                        sync_selection_for_release(
                                                            &rel_copy,
                                                            inst.as_ref(),
                                                            selected_platform,
                                                            selected_asset_name,
                                                        );
                                                    }),
                                                    key: DiffKey::None,
                                                }
                                                .key((rel.id, plat))
                                            }))
                                    })),
                            )
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .height(Size::flex(1.0))
                                    .overflow(Overflow::Clip)
                                    .child(
                                        ScrollView::new().child(
                                            rect()
                                                .width(Size::fill())
                                                .vertical()
                                                .spacing(12.0)
                                                .padding(Gaps::new(0.0, 4.0, 14.0, 0.0))
                                                .child(
                                                    rect()
                                                        .horizontal()
                                                        .cross_align(Alignment::Center)
                                                        .spacing(8.0)
                                                        .child(
                                                            label()
                                                                .text(rel.display_title().to_string())
                                                                .color(theme::TEXT_PRIMARY)
                                                                .font_size(13.5)
                                                                .font_weight(FontWeight::SEMI_BOLD),
                                                        )
                                                        .child(
                                                            label()
                                                                .text("Grouped by Minecraft version")
                                                                .color(theme::TEXT_MUTED)
                                                                .font_size(12.5),
                                                        ),
                                                )
                                                .children((0..row_count).map(|row_idx| {
                                                    let base = row_idx * 3;
                                                    rect()
                                                        .width(Size::fill())
                                                        .horizontal()
                                                        .content(Content::Flex)
                                                        .spacing(12.0)
                                                        .children((0..3).map(|col_idx| {
                                                            if let Some(asset) =
                                                                matching_assets.get(base + col_idx)
                                                            {
                                                                let is_sel = active_asset
                                                                    .as_ref()
                                                                    .is_some_and(|a| {
                                                                        a.name == asset.name
                                                                    });
                                                                let matches_inst = asset
                                                                    .is_explicit_mc_version
                                                                    && detected_mcs.iter().any(
                                                                        |m| m == &asset.mc_version,
                                                                    );
                                                                let ver_lbl = if asset
                                                                    .is_explicit_mc_version
                                                                {
                                                                    asset.mc_version.clone()
                                                                } else {
                                                                    rel.tag_name.clone()
                                                                };
                                                                let (b_id, b_bytes) =
                                                                    theme::banner_for_version(
                                                                        &asset.mc_version,
                                                                    );
                                                                let asset_name =
                                                                    asset.name.clone();
                                                                let badge = if matches_inst {
                                                                    Some("Matches Instance".into())
                                                                } else if cur_release_idx == 0
                                                                    && base + col_idx == 0
                                                                {
                                                                    Some("Latest".into())
                                                                } else {
                                                                    None
                                                                };

                                                                ArtworkCard {
                                                                    banner_id: b_id,
                                                                    banner_bytes: b_bytes,
                                                                    top_badge: badge,
                                                                    title: ver_lbl.clone(),
                                                                    subtitle: format!(
                                                                        "{}  ·  {}  ·  {}",
                                                                        if asset
                                                                            .is_explicit_mc_version
                                                                        {
                                                                            &asset.mc_version
                                                                        } else {
                                                                            "Universal"
                                                                        },
                                                                        asset.platform.label(),
                                                                        asset.formatted_size()
                                                                    ),
                                                                    subtitle_warn: false,
                                                                    selected: is_sel,
                                                                    quick_action_label: None,
                                                                    anim_group: anim_group.clone(),
                                                                    index: base + col_idx,
                                                                    on_select: NoArgCallback::new(
                                                                        move || {
                                                                            selected_asset_name.set(
                                                                                Some(
                                                                                    asset_name
                                                                                        .clone(),
                                                                                ),
                                                                            );
                                                                            install_state.set(
                                                                                InstallState::Idle,
                                                                            );
                                                                        },
                                                                    ),
                                                                    on_quick_action: None,
                                                                    key: DiffKey::None,
                                                                }
                                                                .key(asset.name.clone())
                                                                .into_element()
                                                            } else {
                                                                rect()
                                                                    .width(Size::flex(1.0))
                                                                    .height(Size::px(1.0))
                                                                    .into_element()
                                                            }
                                                        }))
                                                }))
                                                .maybe_child((list.len() > top_tabs_count).then(
                                                    || {
                                                        rect()
                                                            .width(Size::fill())
                                                            .vertical()
                                                            .spacing(8.0)
                                                            .padding(Gaps::new(
                                                                10.0, 0.0, 0.0, 0.0,
                                                            ))
                                                            .child(
                                                                label()
                                                                    .text("Older Releases")
                                                                    .color(theme::TEXT_SECONDARY)
                                                                    .font_size(13.0)
                                                                    .font_weight(
                                                                        FontWeight::SEMI_BOLD,
                                                                    ),
                                                            )
                                                            .child(
                                                                rect()
                                                                    .width(Size::fill())
                                                                    .horizontal()
                                                                    .content(
                                                                        Content::wrap_spacing(8.0),
                                                                    )
                                                                    .spacing(8.0)
                                                                    .children(
                                                                        list.iter()
                                                                            .enumerate()
                                                                            .skip(top_tabs_count)
                                                                            .map(|(idx, old)| {
                                                                                let old_c =
                                                                                    old.clone();
                                                                                ActionButton::new(
                                                                                    old.tag_name
                                                                                        .clone(),
                                                                                    move || {
                                                                                        selected_release_idx
                                                                                            .set(idx);
                                                                                        install_state
                                                                                            .set(InstallState::Idle);
                                                                                        let inst = selected_instance
                                                                                            .peek()
                                                                                            .clone();
                                                                                        sync_selection_for_release(
                                                                                            &old_c,
                                                                                            inst.as_ref(),
                                                                                            selected_platform,
                                                                                            selected_asset_name,
                                                                                        );
                                                                                    },
                                                                                )
                                                                                .variant(
                                                                                    if idx == cur_release_idx {
                                                                                        ButtonVariant::Primary
                                                                                    } else {
                                                                                        ButtonVariant::Control
                                                                                    },
                                                                                )
                                                                                .compact(true)
                                                                            }),
                                                                    ),
                                                            )
                                                    },
                                                )),
                                        ),
                                    ),
                            )
                            .into_element()
                    }
                };

                rect()
                    .expanded()
                    .vertical()
                    .content(Content::Flex)
                    .spacing(16.0)
                    .child(header_row)
                    .child(content_area)
                    .into_element()
            }

            WizardStep::NameJar => {
                let default_jar = active_asset
                    .as_ref()
                    .map(|a| a.name.as_str())
                    .unwrap_or("marlowwwclient-5.5-26.3.jar");
                let has_custom_name = !cur_custom_name.trim().is_empty();
                let preview_file = sanitize_jar_filename(&cur_custom_name, default_jar);
                let preview_ingame = effective_in_game_name(&cur_custom_name);

                let header_row = page_header_row(
                    "Renaming",
                    "Set a custom name for the JAR file and fabric.mod.json so it appears like that in game.",
                    has_custom_name.then(|| {
                        ActionButton::new("Reset Default", move || {
                            custom_jar_name.set(String::new());
                            shell.save_prefs(true);
                        })
                        .icon(theme::PHOSPHOR_CLOSE)
                        .variant(ButtonVariant::Control)
                    }),
                );

                let preset_rows = RENAME_PRESETS.len().div_ceil(3);

                rect()
                    .expanded()
                    .vertical()
                    .content(Content::Flex)
                    .spacing(18.0)
                    .child(header_row)
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::flex(1.0))
                            .overflow(Overflow::Clip)
                            .child(
                                ScrollView::new().child(
                                    rect()
                                        .width(Size::fill())
                                        .vertical()
                                        .spacing(18.0)
                                        .padding(Gaps::new(0.0, 4.0, 14.0, 0.0))
                                        .child(
                                            rect()
                                                .width(Size::fill())
                                                .vertical()
                                                .spacing(8.0)
                                                .child(
                                                    label()
                                                        .text("Custom mod & JAR name")
                                                        .color(theme::TEXT_PRIMARY)
                                                        .font_size(13.0)
                                                        .font_weight(FontWeight::SEMI_BOLD),
                                                )
                                                .child(
                                                    Input::new(custom_jar_name)
                                                        .width(Size::fill())
                                                        .placeholder(
                                                            "Leave empty for Marlowww Client, or enter a custom name...",
                                                        )
                                                        .theme_colors(custom_input_colors())
                                                        .leading(PhosphorIcon::new(
                                                            theme::PHOSPHOR_PENCIL,
                                                            15.0,
                                                            theme::TEXT_SECONDARY,
                                                        )),
                                                ),
                                        )
                                        .child(
                                            rect()
                                                .width(Size::fill())
                                                .vertical()
                                                .spacing(10.0)
                                                .child(
                                                    rect()
                                                        .horizontal()
                                                        .cross_align(Alignment::Center)
                                                        .spacing(8.0)
                                                        .child(
                                                            label()
                                                                .text("Quick Presets")
                                                                .color(theme::TEXT_PRIMARY)
                                                                .font_size(13.5)
                                                                .font_weight(FontWeight::SEMI_BOLD),
                                                        )
                                                        .child(
                                                            label()
                                                                .text("Click any preset to apply")
                                                                .color(theme::TEXT_MUTED)
                                                                .font_size(12.5),
                                                        ),
                                                )
                                                .children((0..preset_rows).map(|r_idx| {
                                                    let base = r_idx * 3;
                                                    rect()
                                                        .width(Size::fill())
                                                        .horizontal()
                                                        .content(Content::Flex)
                                                        .spacing(10.0)
                                                        .children((0..3).map(|c_idx| {
                                                            let (lbl, val) =
                                                                RENAME_PRESETS[base + c_idx];
                                                            let jar_p = sanitize_jar_filename(
                                                                val,
                                                                default_jar,
                                                            );
                                                            DisguisePresetCard {
                                                                title: lbl.to_string(),
                                                                jar_preview: jar_p,
                                                                selected: cur_custom_name.trim()
                                                                    == val,
                                                                index: base + c_idx,
                                                                on_select: NoArgCallback::new(
                                                                    move || {
                                                                        custom_jar_name
                                                                            .set(val.to_string());
                                                                        shell.save_prefs(true);
                                                                    },
                                                                ),
                                                                key: DiffKey::None,
                                                            }
                                                            .key(lbl)
                                                        }))
                                                })),
                                        )
                                        .child(
                                            rect()
                                                .width(Size::fill())
                                                .vertical()
                                                .spacing(10.0)
                                                .padding(Gaps::new(16.0, 18.0, 16.0, 18.0))
                                                .corner_radius(theme::RADIUS_CARD)
                                                .background(theme::BG_PANEL)
                                                .border(
                                                    Border::new()
                                                        .width(1.0)
                                                        .fill(theme::BORDER_SUBTLE),
                                                )
                                                .child(
                                                    rect()
                                                        .width(Size::fill())
                                                        .horizontal()
                                                        .main_align(Alignment::SpaceBetween)
                                                        .cross_align(Alignment::Center)
                                                        .child(
                                                            label()
                                                                .text("fabric.mod.json & File Preview")
                                                                .color(theme::TEXT_PRIMARY)
                                                                .font_size(13.5)
                                                                .font_weight(FontWeight::SEMI_BOLD),
                                                        )
                                                        .child(
                                                            PhosphorIcon::new(
                                                                theme::PHOSPHOR_PENCIL,
                                                                15.0,
                                                                theme::ACCENT_GLOW,
                                                            ),
                                                        ),
                                                )
                                                .child(
                                                    label()
                                                        .text(format!(
                                                            "mods/{preview_file}   ->   fabric.mod.json \"name\": \"{preview_ingame}\""
                                                        ))
                                                        .font_family("Geist Mono")
                                                        .color(Color::from_rgb(184, 198, 255))
                                                        .font_size(12.5),
                                                )
                                                .child(
                                                    label()
                                                        .text("Patches fabric.mod.json inside the JAR during installation so Minecraft and Mod Menu display your chosen name in game.")
                                                        .color(theme::TEXT_SECONDARY)
                                                        .font_size(12.0),
                                                ),
                                        ),
                                ),
                            ),
                    )
                    .into_element()
            }

            WizardStep::SelectInstance => {
                let filtered: Vec<InstanceInfo> = detected_snapshot
                    .iter()
                    .filter(|inst| {
                        cur_filter.matches(inst.launcher) && inst.matches_search(&cur_search)
                    })
                    .cloned()
                    .collect();

                let current_game_dir = instance_snapshot.as_ref().map(|i| i.game_dir.clone());
                let row_count = filtered.len().div_ceil(3);
                let anim_group = format!("inst-{:?}", cur_filter);

                let header_row = page_header_row(
                    "Instances",
                    "Pick a OneClient, Modrinth, Prism, or official Minecraft instance.",
                    Some(
                        ActionButton::new("Browse folder", move || {
                            let start_dir = selected_instance
                                .peek()
                                .as_ref()
                                .map(|i| i.game_dir.clone());
                            spawn_in_window(async move {
                                let picked = thread(move || {
                                    let mut dialog = rfd::FileDialog::new()
                                        .set_title("Select Minecraft Instance Folder");
                                    if let Some(dir) = start_dir {
                                        if dir.is_dir() {
                                            dialog = dialog.set_directory(dir);
                                        }
                                    }
                                    dialog.pick_folder()
                                })
                                .await;
                                if let Some(path) = picked {
                                    shell.apply_instance(path);
                                }
                            });
                        })
                        .icon(theme::PHOSPHOR_PLUS)
                        .variant(ButtonVariant::Primary),
                    ),
                );

                let grid_area: Element = if filtered.is_empty() {
                    rect()
                        .width(Size::fill())
                        .height(Size::fill())
                        .center()
                        .vertical()
                        .spacing(10.0)
                        .padding(Gaps::new_all(24.0))
                        .child(PhosphorIcon::new(
                            theme::PHOSPHOR_FOLDER,
                            28.0,
                            theme::TEXT_MUTED,
                        ))
                        .child(
                            label()
                                .text("No matching instances found")
                                .color(theme::TEXT_PRIMARY)
                                .font_size(14.5)
                                .font_weight(FontWeight::SEMI_BOLD),
                        )
                        .child(
                            label()
                                .text("Clear your search filter, click Browse folder, or drop a Minecraft folder into the window.")
                                .color(theme::TEXT_SECONDARY)
                                .font_size(12.5),
                        )
                        .into_element()
                } else {
                    ScrollView::new()
                        .child(
                            rect()
                                .width(Size::fill())
                                .vertical()
                                .spacing(10.0)
                                .padding(Gaps::new(0.0, 4.0, 14.0, 0.0))
                                .children((0..row_count).map(|r_idx| {
                                    let base = r_idx * 3;
                                    rect()
                                        .width(Size::fill())
                                        .horizontal()
                                        .content(Content::Flex)
                                        .spacing(12.0)
                                        .children((0..3).map(|c_idx| {
                                            if let Some(inst) = filtered.get(base + c_idx) {
                                                let inst_clone = inst.clone();
                                                let inst_upd = inst.clone();
                                                let is_sel = current_game_dir.as_ref()
                                                    == Some(&inst.game_dir);
                                                let mc_ver = inst
                                                    .fabric
                                                    .mc_versions()
                                                    .first()
                                                    .map(|s| s.as_str())
                                                    .unwrap_or("1.21.11");
                                                let (b_id, b_bytes) =
                                                    theme::banner_for_version(mc_ver);

                                                let has_fab = inst.fabric.is_detected();
                                                let mut sub = if has_fab {
                                                    if let Some(v) =
                                                        inst.fabric.mc_versions().first()
                                                    {
                                                        format!("{v}  ·  Fabric")
                                                    } else {
                                                        "Fabric".to_string()
                                                    }
                                                } else {
                                                    "Missing Fabric".to_string()
                                                };
                                                if let Some(m) = inst.primary_installed_mod() {
                                                    if let Some(ver) = &m.version {
                                                        sub.push_str(&format!("  ·  v{ver}"));
                                                    } else {
                                                        sub.push_str("  ·  Installed");
                                                    }
                                                }

                                                let can_upd = inst.has_installed_mod()
                                                    && has_fab
                                                    && !loaded_releases.is_empty()
                                                    && !install_snapshot.is_busy();
                                                let upd_lbl = if can_upd {
                                                    Some(
                                                        if is_instance_outdated(
                                                            inst,
                                                            &loaded_releases,
                                                        ) {
                                                            "Update"
                                                        } else {
                                                            "Update"
                                                        },
                                                    )
                                                } else {
                                                    None
                                                };

                                                ArtworkCard {
                                                    banner_id: b_id,
                                                    banner_bytes: b_bytes,
                                                    top_badge: Some(
                                                        inst.launcher.label().to_string(),
                                                    ),
                                                    title: inst.display_name.clone(),
                                                    subtitle: sub,
                                                    subtitle_warn: !has_fab,
                                                    selected: is_sel,
                                                    quick_action_label: upd_lbl,
                                                    anim_group: anim_group.clone(),
                                                    index: base + c_idx,
                                                    on_select: NoArgCallback::new(move || {
                                                        let mcs = inst_clone
                                                            .fabric
                                                            .mc_versions()
                                                            .to_vec();
                                                        selected_instance
                                                            .set(Some(inst_clone.clone()));
                                                        install_state.set(InstallState::Idle);
                                                        let matched_name = {
                                                            let rel_guard = releases_state.peek();
                                                            let idx =
                                                                *selected_release_idx.peek();
                                                            let plat =
                                                                *selected_platform.peek();
                                                            if let ReleasesState::Loaded(list) =
                                                                &*rel_guard
                                                            {
                                                                list.get(idx).and_then(|rel| {
                                                                    rel.assets_for_platform(plat)
                                                                        .into_iter()
                                                                        .find(|a| {
                                                                            a.is_explicit_mc_version
                                                                                && mcs.iter().any(
                                                                                    |m| {
                                                                                        m == &a
                                                                                            .mc_version
                                                                                    },
                                                                                )
                                                                        })
                                                                        .map(|a| a.name.clone())
                                                                })
                                                            } else {
                                                                None
                                                            }
                                                        };
                                                        if let Some(name) = matched_name {
                                                            selected_asset_name.set(Some(name));
                                                        }
                                                    }),
                                                    on_quick_action: can_upd.then(|| {
                                                        NoArgCallback::new(move || {
                                                            shell.trigger_quick_update(
                                                                inst_upd.clone(),
                                                            );
                                                        })
                                                    }),
                                                    key: DiffKey::None,
                                                }
                                                .key(inst.game_dir.to_string_lossy().to_string())
                                                .into_element()
                                            } else {
                                                rect()
                                                    .width(Size::flex(1.0))
                                                    .height(Size::px(1.0))
                                                    .into_element()
                                            }
                                        }))
                                })),
                        )
                        .into_element()
                };

                rect()
                    .expanded()
                    .vertical()
                    .content(Content::Flex)
                    .spacing(14.0)
                    .child(header_row)
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .content(Content::Flex)
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::End)
                            .border(
                                Border::new()
                                    .width(BorderWidth {
                                        top: 0.0,
                                        right: 0.0,
                                        bottom: 1.0,
                                        left: 0.0,
                                    })
                                    .fill(theme::DIVIDER),
                            )
                            .child(rect().horizontal().spacing(18.0).children(
                                LauncherFilter::ALL.iter().filter_map(|&flt| {
                                    let cnt = detected_snapshot
                                        .iter()
                                        .filter(|i| flt.matches(i.launcher))
                                        .count();
                                    if flt != LauncherFilter::All && cnt == 0 && cur_filter != flt {
                                        return None;
                                    }
                                    let lbl = if flt == LauncherFilter::All {
                                        format!("All ({cnt})")
                                    } else {
                                        flt.label().to_string()
                                    };
                                    Some(
                                        UnderlineTab {
                                            label: lbl,
                                            selected: cur_filter == flt,
                                            on_press: NoArgCallback::new(move || {
                                                launcher_filter.set(flt);
                                            }),
                                            key: DiffKey::None,
                                        }
                                        .key(flt.label()),
                                    )
                                }),
                            ))
                            .child(
                                rect()
                                    .width(Size::px(244.0))
                                    .margin(Gaps::new(0.0, 0.0, 6.0, 0.0))
                                    .child(
                                        Input::new(instance_search)
                                            .width(Size::fill())
                                            .compact()
                                            .placeholder("Search instances")
                                            .theme_colors(custom_input_colors())
                                            .leading(PhosphorIcon::new(
                                                theme::PHOSPHOR_SEARCH,
                                                14.0,
                                                theme::TEXT_MUTED,
                                            )),
                                    ),
                            ),
                    )
                    .maybe_child(
                        instance_snapshot
                            .as_ref()
                            .and_then(|inst| match &inst.fabric {
                                FabricStatus::Missing { reason } => {
                                    let path_rescan = inst.selected_path.clone();
                                    Some(
                                        rect()
                                            .width(Size::fill())
                                            .horizontal()
                                            .content(Content::Flex)
                                            .cross_align(Alignment::Center)
                                            .spacing(10.0)
                                            .padding(Gaps::new(10.0, 14.0, 10.0, 14.0))
                                            .corner_radius(theme::RADIUS_CONTROL)
                                            .background(theme::ERR_BG)
                                            .border(
                                                Border::new().width(1.0).fill(theme::ERR_BORDER),
                                            )
                                            .child(PhosphorIcon::new(
                                                theme::PHOSPHOR_WARNING,
                                                15.0,
                                                theme::STATUS_ERR,
                                            ))
                                            .child(
                                                label()
                                                    .width(Size::flex(1.0))
                                                    .text(reason.clone())
                                                    .color(theme::STATUS_ERR)
                                                    .font_size(12.0)
                                                    .max_lines(1)
                                                    .text_overflow(TextOverflow::Ellipsis),
                                            )
                                            .child(
                                                ActionButton::new("Get Fabric", || {
                                                    let _ = open::that(
                                                        "https://fabricmc.net/use/installer/",
                                                    );
                                                })
                                                .trailing_icon(theme::PHOSPHOR_EXTERNAL)
                                                .variant(ButtonVariant::Control)
                                                .compact(true),
                                            )
                                            .child(
                                                ActionButton::new("Rescan", move || {
                                                    shell.apply_instance(path_rescan.clone());
                                                })
                                                .variant(ButtonVariant::Subtle)
                                                .compact(true),
                                            ),
                                    )
                                }
                                _ => None,
                            }),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::flex(1.0))
                            .overflow(Overflow::Clip)
                            .child(grid_area),
                    )
                    .into_element()
            }

            WizardStep::Install => {
                let default_jar = active_asset
                    .as_ref()
                    .map(|a| a.name.as_str())
                    .unwrap_or("marlowwwclient.jar");
                let final_jar_name = sanitize_jar_filename(&cur_custom_name, default_jar);
                let final_ingame_name = effective_in_game_name(&cur_custom_name);

                let rel_summary = match (&active_release, &active_asset) {
                    (Some(r), Some(a)) => format!(
                        "{}  ·  {}  ·  MC {}",
                        r.display_title(),
                        a.platform.label(),
                        a.mc_version
                    ),
                    _ => "No release build selected".to_string(),
                };

                let inst_summary = match &instance_snapshot {
                    Some(i) => format!(
                        "{} ({})  ·  {}",
                        i.display_name,
                        i.launcher.label(),
                        i.mods_dir.to_string_lossy()
                    ),
                    None => "No instance selected yet".to_string(),
                };

                let status_banner: Option<Element> = match &install_snapshot {
                    InstallState::Idle => None,
                    InstallState::Downloading {
                        file_name,
                        downloaded,
                        total,
                        speed_bps,
                    } => {
                        let pct = if *total > 0 {
                            (*downloaded as f32 / *total as f32 * 100.0).clamp(0.0, 100.0)
                        } else {
                            0.0
                        };
                        let speed = if *speed_bps > 0.0 {
                            format!("  ·  {}/s", format_bytes(*speed_bps as u64))
                        } else {
                            String::new()
                        };
                        Some(
                            rect()
                                .width(Size::fill())
                                .vertical()
                                .spacing(10.0)
                                .padding(Gaps::new(16.0, 18.0, 16.0, 18.0))
                                .corner_radius(theme::RADIUS_CARD)
                                .background(theme::BG_PANEL)
                                .border(Border::new().width(1.0).fill(theme::ACCENT_RING))
                                .child(
                                    rect()
                                        .width(Size::fill())
                                        .horizontal()
                                        .main_align(Alignment::SpaceBetween)
                                        .cross_align(Alignment::Center)
                                        .child(
                                            label()
                                                .text(format!("Downloading {file_name}"))
                                                .color(theme::TEXT_PRIMARY)
                                                .font_size(14.0)
                                                .font_weight(FontWeight::SEMI_BOLD),
                                        )
                                        .child(
                                            label()
                                                .text(format!(
                                                    "{} / {}{}",
                                                    format_bytes(*downloaded),
                                                    format_bytes(*total),
                                                    speed
                                                ))
                                                .font_family("Geist Mono")
                                                .font_tabular()
                                                .color(theme::TEXT_SECONDARY)
                                                .font_size(12.0),
                                        ),
                                )
                                .child(SlimProgressBar { percent: pct })
                                .into_element(),
                        )
                    }
                    InstallState::Finishing { file_name } => Some(
                        rect()
                            .width(Size::fill())
                            .vertical()
                            .spacing(10.0)
                            .padding(Gaps::new(16.0, 18.0, 16.0, 18.0))
                            .corner_radius(theme::RADIUS_CARD)
                            .background(theme::BG_PANEL)
                            .border(Border::new().width(1.0).fill(theme::ACCENT_RING))
                            .child(
                                label()
                                    .text(format!("Configuring {file_name} in mods/..."))
                                    .color(theme::TEXT_PRIMARY)
                                    .font_size(14.0)
                                    .font_weight(FontWeight::SEMI_BOLD),
                            )
                            .child(SlimProgressBar { percent: 100.0 })
                            .into_element(),
                    ),
                    InstallState::Success {
                        file_name,
                        in_game_name,
                        dest_path,
                        removed_old,
                    } => {
                        let folder = dest_path
                            .parent()
                            .map(|p| p.to_path_buf())
                            .unwrap_or_else(|| dest_path.clone());
                        let note = if removed_old.is_empty() {
                            format!("Installed {file_name} (in-game: {in_game_name}).")
                        } else {
                            format!(
                                "Installed {file_name} (in-game: {in_game_name}) and replaced {} previous build{}.",
                                removed_old.len(),
                                if removed_old.len() == 1 { "" } else { "s" }
                            )
                        };
                        Some(
                            rect()
                                .width(Size::fill())
                                .vertical()
                                .spacing(12.0)
                                .padding(Gaps::new(18.0, 20.0, 18.0, 20.0))
                                .corner_radius(theme::RADIUS_CARD)
                                .background(theme::ACCENT_SOFT)
                                .border(Border::new().width(1.0).fill(theme::ACCENT_RING))
                                .child(
                                    rect()
                                        .horizontal()
                                        .cross_align(Alignment::Center)
                                        .spacing(9.0)
                                        .child(PhosphorIcon::new(
                                            theme::PHOSPHOR_CHECK,
                                            17.0,
                                            theme::STATUS_OK,
                                        ))
                                        .child(
                                            label()
                                                .text("Installation Complete")
                                                .color(theme::TEXT_PRIMARY)
                                                .font_size(15.5)
                                                .font_weight(FontWeight::BOLD),
                                        ),
                                )
                                .child(
                                    label()
                                        .text("Marlowww Client is done installing. Launch your game from your launcher as normal.")
                                        .color(theme::TEXT_PRIMARY)
                                        .font_size(13.0),
                                )
                                .child(
                                    label()
                                        .text(note)
                                        .font_family("Geist Mono")
                                        .color(Color::from_rgb(184, 198, 255))
                                        .font_size(12.0),
                                )
                                .child(
                                    rect()
                                        .horizontal()
                                        .spacing(8.0)
                                        .child(
                                            ActionButton::new("Open mods folder", move || {
                                                let _ = open::that(&folder);
                                            })
                                            .icon(theme::PHOSPHOR_FOLDER)
                                            .variant(ButtonVariant::Control),
                                        )
                                        .child(
                                            ActionButton::new("Pick another instance", move || {
                                                install_state.set(InstallState::Idle);
                                                wizard_step.set(WizardStep::SelectInstance);
                                            })
                                            .variant(ButtonVariant::Subtle),
                                        ),
                                )
                                .into_element(),
                        )
                    }
                    InstallState::Error(err) => Some(
                        rect()
                            .width(Size::fill())
                            .vertical()
                            .spacing(8.0)
                            .padding(Gaps::new(16.0, 18.0, 16.0, 18.0))
                            .corner_radius(theme::RADIUS_CARD)
                            .background(theme::ERR_BG)
                            .border(Border::new().width(1.0).fill(theme::ERR_BORDER))
                            .child(
                                label()
                                    .text("Installation failed")
                                    .color(theme::STATUS_ERR)
                                    .font_size(14.0)
                                    .font_weight(FontWeight::SEMI_BOLD),
                            )
                            .child(
                                label()
                                    .text(err.clone())
                                    .color(theme::TEXT_PRIMARY)
                                    .font_size(12.5),
                            )
                            .into_element(),
                    ),
                };

                rect()
                    .expanded()
                    .vertical()
                    .content(Content::Flex)
                    .spacing(16.0)
                    .child(page_header_row(
                        "Review & Install",
                        "Confirm your configuration below, then click Install Client on the right.",
                        None,
                    ))
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::flex(1.0))
                            .overflow(Overflow::Clip)
                            .child(
                                ScrollView::new().child(
                                    rect()
                                        .width(Size::fill())
                                        .vertical()
                                        .spacing(12.0)
                                        .padding(Gaps::new(0.0, 4.0, 14.0, 0.0))
                                        .maybe_child(status_banner)
                                        .child(summary_card(
                                            "Version & Platform",
                                            &rel_summary,
                                            "Edit",
                                            move || wizard_step.set(WizardStep::SelectVersion),
                                        ))
                                        .child(summary_card(
                                            "Renaming",
                                            &format!(
                                                "{final_jar_name}  ·  In-game: {final_ingame_name}"
                                            ),
                                            "Edit",
                                            move || wizard_step.set(WizardStep::NameJar),
                                        ))
                                        .child(summary_card(
                                            "Target Instance",
                                            &inst_summary,
                                            "Edit",
                                            move || wizard_step.set(WizardStep::SelectInstance),
                                        ))
                                        .child(
                                            rect()
                                                .width(Size::fill())
                                                .horizontal()
                                                .content(Content::Flex)
                                                .cross_align(Alignment::Center)
                                                .spacing(14.0)
                                                .padding(Gaps::new(14.0, 18.0, 14.0, 18.0))
                                                .corner_radius(theme::RADIUS_CARD)
                                                .background(theme::BG_PANEL)
                                                .border(
                                                    Border::new()
                                                        .width(1.0)
                                                        .fill(theme::BORDER_SUBTLE),
                                                )
                                                .child(
                                                    rect()
                                                        .width(Size::flex(1.0))
                                                        .vertical()
                                                        .spacing(3.0)
                                                        .child(
                                                            label()
                                                                .text("Replace older Marlowww Client builds")
                                                                .color(theme::TEXT_PRIMARY)
                                                                .font_size(13.5)
                                                                .font_weight(FontWeight::SEMI_BOLD),
                                                        )
                                                        .child(
                                                            label()
                                                                .text("Removes older or renamed Marlowww Client jars in mods/ automatically.")
                                                                .color(theme::TEXT_SECONDARY)
                                                                .font_size(12.0),
                                                        ),
                                                )
                                                .child(ToggleSwitch {
                                                    checked: replace_existing(),
                                                    on_toggle: NoArgCallback::new(move || {
                                                        let cur = *replace_existing.peek();
                                                        replace_existing.set(!cur);
                                                        shell.save_prefs(true);
                                                    }),
                                                }),
                                        ),
                                ),
                            ),
                    )
                    .into_element()
            }
        };

        let inspector_title = match (&instance_snapshot, cur_step) {
            (Some(inst), WizardStep::SelectInstance | WizardStep::Install) => {
                inst.display_name.clone()
            }
            _ => version_codename(active_mc_ver).to_string(),
        };

        let inspector_badge = match (&instance_snapshot, cur_step) {
            (Some(inst), WizardStep::SelectInstance | WizardStep::Install) => {
                inst.launcher.label().to_string()
            }
            _ => active_release
                .as_ref()
                .map(|r| r.tag_name.clone())
                .unwrap_or_else(|| "Marlowww".to_string()),
        };

        let inspector_subtitle = format!(
            "{}  ·  {}",
            active_mc_ver,
            active_asset
                .as_ref()
                .map(|a| a.platform.label())
                .unwrap_or("Desktop")
        );

        let default_jar_for_insp = active_asset
            .as_ref()
            .map(|a| a.name.as_str())
            .unwrap_or("marlowwwclient.jar");
        let insp_jar_name = sanitize_jar_filename(&cur_custom_name, default_jar_for_insp);
        let insp_mod_name = effective_in_game_name(&cur_custom_name);
        let insp_instance_val = instance_snapshot
            .as_ref()
            .map(|i| i.display_name.clone())
            .unwrap_or_else(|| "None selected".to_string());
        let insp_loader_val = match &instance_snapshot {
            Some(i) if i.fabric.is_detected() => "Fabric".to_string(),
            Some(_) => "Missing Fabric".to_string(),
            None => "Required".to_string(),
        };
        let insp_key = format!("{:?}-{}-{}", cur_step, active_mc_ver, insp_instance_val);

        let has_installed_on_selected = instance_snapshot
            .as_ref()
            .is_some_and(|i| i.has_installed_mod() && i.fabric.is_detected());
        let mods_dir_for_btn = instance_snapshot.as_ref().map(|i| i.mods_dir.clone());

        let dock_primary = match cur_step {
            WizardStep::SelectVersion => ActionButton::new("Continue", move || {
                wizard_step.set(WizardStep::NameJar);
            })
            .action_tag("dock-step-1")
            .variant(ButtonVariant::Primary)
            .flex_width(true)
            .disabled(active_asset.is_none()),

            WizardStep::NameJar => ActionButton::new("Continue", move || {
                shell.save_prefs(true);
                wizard_step.set(WizardStep::SelectInstance);
            })
            .action_tag("dock-step-2")
            .variant(ButtonVariant::Primary)
            .flex_width(true),

            WizardStep::SelectInstance => ActionButton::new("Continue", move || {
                wizard_step.set(WizardStep::Install);
            })
            .action_tag("dock-step-3")
            .variant(ButtonVariant::Primary)
            .flex_width(true)
            .disabled(!fabric_ready),

            WizardStep::Install => ActionButton::new(
                if install_snapshot.is_busy() {
                    "Installing..."
                } else {
                    "Install Client"
                },
                move || {
                    shell.install_current_selection();
                },
            )
            .action_tag("dock-step-4")
            .icon(theme::PHOSPHOR_DOWNLOAD)
            .variant(ButtonVariant::Primary)
            .flex_width(true)
            .disabled(!can_install),
        };

        let right_inspector = rect()
            .width(Size::fill())
            .height(Size::fill())
            .corner_radius(theme::RADIUS_PANEL)
            .overflow(Overflow::Clip)
            .background(theme::BG_PANEL)
            .border(Border::new().width(1.0).fill(theme::BORDER_SUBTLE))
            .vertical()
            .content(Content::Flex)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(148.0))
                    .overflow(Overflow::Clip)
                    .child(
                        ImageViewer::new((hero_banner_id, hero_banner_bytes))
                            .width(Size::fill())
                            .height(Size::fill())
                            .aspect_ratio(AspectRatio::Max)
                            .image_cover(ImageCover::Center),
                    )
                    .child(
                        rect()
                            .position(Position::new_absolute().top(0.0).left(0.0))
                            .padding(Gaps::new(12.0, 14.0, 12.0, 14.0))
                            .child(
                                rect()
                                    .padding(Gaps::new(3.5, 9.5, 3.5, 9.5))
                                    .corner_radius(6.0)
                                    .background(theme::BG_PILL_SCRIM)
                                    .border(
                                        Border::new()
                                            .width(1.0)
                                            .fill(Color::from_argb(26, 255, 255, 255)),
                                    )
                                    .child(
                                        label()
                                            .text(inspector_badge)
                                            .color(theme::TEXT_PRIMARY)
                                            .font_size(11.0)
                                            .font_weight(FontWeight::MEDIUM),
                                    ),
                            ),
                    ),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.0))
                    .padding(Gaps::new(16.0, 18.0, 16.0, 18.0))
                    .vertical()
                    .content(Content::Flex)
                    .main_align(Alignment::SpaceBetween)
                    .child(
                        MotionTransition::new(
                            insp_key,
                            rect()
                                .width(Size::fill())
                                .vertical()
                                .spacing(14.0)
                                .child(
                                    rect()
                                        .width(Size::fill())
                                        .vertical()
                                        .spacing(3.0)
                                        .child(
                                            label()
                                                .width(Size::fill())
                                                .text(inspector_title)
                                                .color(theme::TEXT_PRIMARY)
                                                .font_size(20.0)
                                                .font_weight(FontWeight::BOLD)
                                                .letter_spacing(-0.4)
                                                .max_lines(1)
                                                .text_overflow(TextOverflow::Ellipsis),
                                        )
                                        .child(
                                            label()
                                                .text(inspector_subtitle)
                                                .color(theme::TEXT_SECONDARY)
                                                .font_size(12.5),
                                        ),
                                )
                                .child(
                                    rect()
                                        .width(Size::fill())
                                        .vertical()
                                        .child(inspector_kv_row("Minecraft", active_mc_ver))
                                        .child(inspector_kv_row("Loader", &insp_loader_val))
                                        .child(inspector_kv_row("Mod Name", &insp_mod_name))
                                        .child(inspector_kv_row("JAR File", &insp_jar_name))
                                        .child(inspector_kv_row("Instance", &insp_instance_val)),
                                )
                                .child(
                                    label()
                                        .width(Size::fill())
                                        .text(version_summary_blurb(active_mc_ver))
                                        .color(theme::TEXT_MUTED)
                                        .font_size(12.0)
                                        .max_lines(3),
                                ),
                        )
                        .slide(0.0, 6.0)
                        .height(Size::Inner),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .vertical()
                            .spacing(8.0)
                            .maybe_child(
                                (has_installed_on_selected
                                    && !loaded_releases.is_empty()
                                    && !install_snapshot.is_busy())
                                .then(|| {
                                    ActionButton::new("Update to Latest", move || {
                                        let inst_opt = { shell.selected_instance.peek().clone() };
                                        if let Some(i) = inst_opt {
                                            shell.trigger_quick_update(i);
                                        }
                                    })
                                    .action_tag("dock-update-latest")
                                    .icon(theme::PHOSPHOR_REFRESH)
                                    .variant(ButtonVariant::SecondaryTint)
                                    .fill_width(true)
                                }),
                            )
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .horizontal()
                                    .content(Content::Flex)
                                    .cross_align(Alignment::Center)
                                    .spacing(8.0)
                                    .child(dock_primary)
                                    .maybe_child((cur_step != WizardStep::SelectVersion).then(
                                        || IconSquareButton {
                                            icon: theme::PHOSPHOR_ARROW_LEFT,
                                            disabled: false,
                                            on_press: NoArgCallback::new(move || {
                                                let cur = { *wizard_step.peek() };
                                                let prev = match cur {
                                                    WizardStep::SelectVersion => {
                                                        WizardStep::SelectVersion
                                                    }
                                                    WizardStep::NameJar => {
                                                        WizardStep::SelectVersion
                                                    }
                                                    WizardStep::SelectInstance => {
                                                        WizardStep::NameJar
                                                    }
                                                    WizardStep::Install => {
                                                        WizardStep::SelectInstance
                                                    }
                                                };
                                                wizard_step.set(prev);
                                            }),
                                        },
                                    ))
                                    .child(IconSquareButton {
                                        icon: theme::PHOSPHOR_FOLDER,
                                        disabled: mods_dir_for_btn.is_none(),
                                        on_press: NoArgCallback::new(move || {
                                            if let Some(dir) = &mods_dir_for_btn {
                                                let _ = std::fs::create_dir_all(dir);
                                                let _ = open::that(dir);
                                            }
                                        }),
                                    }),
                            ),
                    ),
            );

        let advance_tour = move || {
            let next = { tour_step.peek().next() };
            if let Some(target_wiz) = next.target_wizard_step() {
                wizard_step.set(target_wiz);
            }
            if next == TourStep::Hidden {
                wizard_step.set(WizardStep::SelectVersion);
                shell.save_prefs(true);
            }
            tour_step.set(next);
        };

        let rewind_tour = move || {
            let prev = { tour_step.peek().prev() };
            if let Some(target_wiz) = prev.target_wizard_step() {
                wizard_step.set(target_wiz);
            }
            tour_step.set(prev);
        };

        let dismiss_tour = move || {
            tour_step.set(TourStep::Hidden);
            shell.save_prefs(true);
        };

        let left_wrapped = SpotlightZoneWrapper::new(
            spot_zone == SpotlightZone::LeftCanvas,
            spot_zone == SpotlightZone::RightInspector,
            14.0,
            Size::flex(1.0),
            Size::fill(),
            theme::RADIUS_PANEL,
            MotionTransition::new(format!("step-{:?}", cur_step), left_canvas_body)
                .slide(12.0, 4.0),
        );

        let right_wrapped = SpotlightZoneWrapper::new(
            spot_zone == SpotlightZone::RightInspector,
            spot_zone == SpotlightZone::LeftCanvas,
            2.0,
            Size::px(308.0),
            Size::fill(),
            theme::RADIUS_PANEL,
            right_inspector,
        );

        rect()
            .expanded()
            .background(theme::canvas_ambient_gradient())
            .color(theme::TEXT_PRIMARY)
            .font_family("Geist")
            .on_global_file_hover(move |_| {
                is_dragging_file.set_if_modified(true);
            })
            .on_global_file_hover_cancelled(move |_| {
                is_dragging_file.set_if_modified(false);
            })
            .on_file_drop(move |e: Event<FileEventData>| {
                is_dragging_file.set_if_modified(false);
                if let Some(first) = e.file_paths.first() {
                    let path = if first.is_dir() {
                        first.clone()
                    } else {
                        first
                            .parent()
                            .map(|p| p.to_path_buf())
                            .unwrap_or_else(|| first.clone())
                    };
                    wizard_step.set(WizardStep::SelectInstance);
                    shell.apply_instance(path);
                }
            })
            .child(
                rect()
                    .position(Position::new_absolute().top(0.0).left(0.0))
                    .width(Size::percent(100.0))
                    .height(Size::px(240.0))
                    .overflow(Overflow::Clip)
                    .opacity(0.16)
                    .child(
                        ImageViewer::new((hero_banner_id, hero_banner_bytes))
                            .width(Size::fill())
                            .height(Size::fill())
                            .aspect_ratio(AspectRatio::Max)
                            .image_cover(ImageCover::Center)
                            .blur(26.0),
                    ),
            )
            .child(
                rect()
                    .expanded()
                    .vertical()
                    .content(Content::Flex)
                    .child(top_bar)
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::flex(1.0))
                            .vertical()
                            .content(Content::Flex)
                            .spacing(12.0)
                            .padding(Gaps::new(14.0, 22.0, 18.0, 24.0))
                            .maybe_child(spotlight_active.then(|| SpotlightCoachBar {
                                step: cur_tour,
                                on_prev: NoArgCallback::new(rewind_tour),
                                on_next: NoArgCallback::new(advance_tour),
                                on_skip: NoArgCallback::new(dismiss_tour),
                            }))
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .height(Size::flex(1.0))
                                    .horizontal()
                                    .content(Content::Flex)
                                    .spacing(20.0)
                                    .child(left_wrapped)
                                    .child(right_wrapped),
                            ),
                    ),
            )
            .child(OnboardingModalOverlay {
                step: cur_tour,
                on_prev: NoArgCallback::new(rewind_tour),
                on_next: NoArgCallback::new(advance_tour),
                on_skip: NoArgCallback::new(dismiss_tour),
            })
            .child(DropBackdrop {
                visible: is_dragging_file(),
            })
    }
}

fn page_header_row(
    title_text: &str,
    subtitle_text: &str,
    trailing_btn: Option<ActionButton>,
) -> Element {
    rect()
        .width(Size::fill())
        .horizontal()
        .content(Content::Flex)
        .main_align(Alignment::SpaceBetween)
        .cross_align(Alignment::End)
        .spacing(16.0)
        .child(
            rect()
                .width(Size::flex(1.0))
                .vertical()
                .spacing(4.0)
                .child(
                    label()
                        .text(title_text.to_string())
                        .color(theme::TEXT_PRIMARY)
                        .font_size(26.0)
                        .font_weight(FontWeight::BOLD)
                        .letter_spacing(-0.6),
                )
                .child(
                    label()
                        .text(subtitle_text.to_string())
                        .color(theme::TEXT_SECONDARY)
                        .font_size(13.0)
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis),
                ),
        )
        .maybe_child(trailing_btn)
        .into_element()
}

fn inspector_kv_row(key_text: &str, val_text: &str) -> Element {
    rect()
        .width(Size::fill())
        .horizontal()
        .content(Content::Flex)
        .main_align(Alignment::SpaceBetween)
        .cross_align(Alignment::Center)
        .spacing(12.0)
        .padding(Gaps::new(9.5, 0.0, 9.5, 0.0))
        .border(
            Border::new()
                .width(BorderWidth {
                    top: 1.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 0.0,
                })
                .fill(theme::DIVIDER),
        )
        .child(
            label()
                .text(key_text.to_string())
                .color(theme::TEXT_MUTED)
                .font_size(12.5),
        )
        .child(
            label()
                .width(Size::flex(1.0))
                .text_align(TextAlign::End)
                .text(val_text.to_string())
                .color(theme::TEXT_PRIMARY)
                .font_size(12.5)
                .font_weight(FontWeight::SEMI_BOLD)
                .max_lines(1)
                .text_overflow(TextOverflow::Ellipsis),
        )
        .into_element()
}

fn summary_card(
    heading: &str,
    value: &str,
    action_label: &str,
    mut on_edit: impl FnMut() + 'static,
) -> Element {
    rect()
        .width(Size::fill())
        .horizontal()
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .spacing(12.0)
        .padding(Gaps::new(13.0, 16.0, 13.0, 16.0))
        .corner_radius(theme::RADIUS_CARD)
        .background(theme::BG_PANEL)
        .border(Border::new().width(1.0).fill(theme::BORDER_SUBTLE))
        .child(
            rect()
                .width(Size::flex(1.0))
                .vertical()
                .spacing(3.0)
                .child(
                    label()
                        .text(heading.to_string())
                        .color(theme::TEXT_MUTED)
                        .font_size(11.5)
                        .font_weight(FontWeight::MEDIUM),
                )
                .child(
                    label()
                        .width(Size::fill())
                        .text(value.to_string())
                        .color(theme::TEXT_PRIMARY)
                        .font_size(13.0)
                        .font_weight(FontWeight::MEDIUM)
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis),
                ),
        )
        .child(
            ActionButton::new(action_label, move || on_edit())
                .variant(ButtonVariant::Control)
                .compact(true),
        )
        .into_element()
}

#[cfg(test)]
mod snapshot_tests {
    use std::{borrow::Cow, collections::HashMap, time::Duration};

    use freya_testing::TestingRunner;

    use super::*;
    use crate::models::{InstalledModInfo, LauncherKind};

    fn sample_releases() -> Vec<ReleaseInfo> {
        vec![
            ReleaseInfo {
                id: 1,
                tag_name: "v5.5-SPOOKEH".to_string(),
                name: "Marlowww Client v5.5".to_string(),
                published_date: "2026-10-30".to_string(),
                prerelease: false,
                html_url: "https://github.com/nxghtCry0/marlowwwclient/releases/tag/v5.5-SPOOKEH"
                    .to_string(),
                assets: vec![
                    ReleaseAsset {
                        name: "marlowwwclient-5.5-26.3.jar".to_string(),
                        download_url: String::new(),
                        size: 19_387_474,
                        platform: PlatformKind::Desktop,
                        mc_version: "26.3".to_string(),
                        is_explicit_mc_version: true,
                    },
                    ReleaseAsset {
                        name: "marlowwwclient-5.5-26.2.jar".to_string(),
                        download_url: String::new(),
                        size: 19_378_574,
                        platform: PlatformKind::Desktop,
                        mc_version: "26.2".to_string(),
                        is_explicit_mc_version: true,
                    },
                    ReleaseAsset {
                        name: "marlowwwclient-5.5-26.1.2.jar".to_string(),
                        download_url: String::new(),
                        size: 19_380_010,
                        platform: PlatformKind::Desktop,
                        mc_version: "26.1.2".to_string(),
                        is_explicit_mc_version: true,
                    },
                    ReleaseAsset {
                        name: "marlowwwclient-5.5-1.21.11.jar".to_string(),
                        download_url: String::new(),
                        size: 19_554_308,
                        platform: PlatformKind::Desktop,
                        mc_version: "1.21.11".to_string(),
                        is_explicit_mc_version: true,
                    },
                    ReleaseAsset {
                        name: "marlowwwclient-5.5-1.21.11-android.jar".to_string(),
                        download_url: String::new(),
                        size: 19_554_308,
                        platform: PlatformKind::Android,
                        mc_version: "1.21.11".to_string(),
                        is_explicit_mc_version: true,
                    },
                ],
            },
            ReleaseInfo {
                id: 2,
                tag_name: "v5".to_string(),
                name: "Marlowww Client V5".to_string(),
                published_date: "2026-09-26".to_string(),
                prerelease: false,
                html_url: String::new(),
                assets: vec![ReleaseAsset {
                    name: "marlowwwclient-v5.jar".to_string(),
                    download_url: String::new(),
                    size: 8_472_282,
                    platform: PlatformKind::Desktop,
                    mc_version: "Default".to_string(),
                    is_explicit_mc_version: false,
                }],
            },
            ReleaseInfo {
                id: 3,
                tag_name: "v4".to_string(),
                name: "Marlowww Client V4".to_string(),
                published_date: "2026-07-22".to_string(),
                prerelease: false,
                html_url: String::new(),
                assets: vec![ReleaseAsset {
                    name: "marlowwwclient-v4.jar".to_string(),
                    download_url: String::new(),
                    size: 2_376_620,
                    platform: PlatformKind::Desktop,
                    mc_version: "Default".to_string(),
                    is_explicit_mc_version: false,
                }],
            },
            ReleaseInfo {
                id: 4,
                tag_name: "v3.1.2".to_string(),
                name: "Marlow Client v3.1.2".to_string(),
                published_date: "2026-06-16".to_string(),
                prerelease: false,
                html_url: String::new(),
                assets: vec![ReleaseAsset {
                    name: "marlowwwclient-3.1.2-1.21.11.jar".to_string(),
                    download_url: String::new(),
                    size: 623_368,
                    platform: PlatformKind::Desktop,
                    mc_version: "1.21.11".to_string(),
                    is_explicit_mc_version: true,
                }],
            },
        ]
    }

    fn sample_instances() -> Vec<InstanceInfo> {
        vec![
            InstanceInfo {
                display_name: "26.3 Fabric".to_string(),
                launcher: LauncherKind::OneClient,
                selected_path: PathBuf::from(
                    r"C:\AppData\Roaming\Polyfrost\OneClient\clusters\26.3 Fabric",
                ),
                game_dir: PathBuf::from(
                    r"C:\AppData\Roaming\Polyfrost\OneClient\clusters\26.3 Fabric",
                ),
                mods_dir: PathBuf::from(
                    r"C:\AppData\Roaming\Polyfrost\OneClient\clusters\26.3 Fabric\mods",
                ),
                fabric: FabricStatus::Detected {
                    loader_version: None,
                    mc_versions: vec!["26.3".to_string()],
                    source: ".instance.json".to_string(),
                },
                installed_mods: vec![InstalledModInfo {
                    file_name: "marlowwwclient-5.2.1-26.3.jar".to_string(),
                    version: Some("5.2.1".to_string()),
                    display_name: Some("Marlowww Client".to_string()),
                    is_android: false,
                }],
            },
            InstanceInfo {
                display_name: "Insanely Optimized".to_string(),
                launcher: LauncherKind::Modrinth,
                selected_path: PathBuf::from(
                    r"C:\AppData\Roaming\ModrinthApp\profiles\Remarkably Optimized (1)",
                ),
                game_dir: PathBuf::from(
                    r"C:\AppData\Roaming\ModrinthApp\profiles\Remarkably Optimized (1)",
                ),
                mods_dir: PathBuf::from(
                    r"C:\AppData\Roaming\ModrinthApp\profiles\Remarkably Optimized (1)\mods",
                ),
                fabric: FabricStatus::Detected {
                    loader_version: Some("0.16.12".to_string()),
                    mc_versions: vec!["1.21.11".to_string()],
                    source: "Modrinth app.db".to_string(),
                },
                installed_mods: vec![InstalledModInfo {
                    file_name: "sodium-extra.jar".to_string(),
                    version: Some("5.0".to_string()),
                    display_name: Some("Sodium Extra".to_string()),
                    is_android: false,
                }],
            },
            InstanceInfo {
                display_name: "Ranked PvP".to_string(),
                launcher: LauncherKind::Prism,
                selected_path: PathBuf::from(r"C:\AppData\Roaming\PrismLauncher\instances\1.21.11"),
                game_dir: PathBuf::from(
                    r"C:\AppData\Roaming\PrismLauncher\instances\1.21.11\.minecraft",
                ),
                mods_dir: PathBuf::from(
                    r"C:\AppData\Roaming\PrismLauncher\instances\1.21.11\.minecraft\mods",
                ),
                fabric: FabricStatus::Detected {
                    loader_version: Some("0.16.10".to_string()),
                    mc_versions: vec!["1.21.11".to_string()],
                    source: "mmc-pack.json".to_string(),
                },
                installed_mods: Vec::new(),
            },
            InstanceInfo {
                display_name: "Crystal Practice".to_string(),
                launcher: LauncherKind::OneClient,
                selected_path: PathBuf::from(
                    r"C:\AppData\Roaming\Polyfrost\OneClient\clusters\1.21.11 Crystal",
                ),
                game_dir: PathBuf::from(
                    r"C:\AppData\Roaming\Polyfrost\OneClient\clusters\1.21.11 Crystal",
                ),
                mods_dir: PathBuf::from(
                    r"C:\AppData\Roaming\Polyfrost\OneClient\clusters\1.21.11 Crystal\mods",
                ),
                fabric: FabricStatus::Detected {
                    loader_version: Some("0.16.12".to_string()),
                    mc_versions: vec!["1.21.11".to_string()],
                    source: ".instance.json".to_string(),
                },
                installed_mods: Vec::new(),
            },
            InstanceInfo {
                display_name: "26.1.2 Modrinth".to_string(),
                launcher: LauncherKind::Modrinth,
                selected_path: PathBuf::from(
                    r"C:\AppData\Roaming\ModrinthApp\profiles\26.1.2 Fast",
                ),
                game_dir: PathBuf::from(r"C:\AppData\Roaming\ModrinthApp\profiles\26.1.2 Fast"),
                mods_dir: PathBuf::from(
                    r"C:\AppData\Roaming\ModrinthApp\profiles\26.1.2 Fast\mods",
                ),
                fabric: FabricStatus::Detected {
                    loader_version: Some("0.16.12".to_string()),
                    mc_versions: vec!["26.1.2".to_string()],
                    source: "Modrinth app.db".to_string(),
                },
                installed_mods: Vec::new(),
            },
            InstanceInfo {
                display_name: "26.2 Arena".to_string(),
                launcher: LauncherKind::Prism,
                selected_path: PathBuf::from(
                    r"C:\AppData\Roaming\PrismLauncher\instances\26.2 test",
                ),
                game_dir: PathBuf::from(
                    r"C:\AppData\Roaming\PrismLauncher\instances\26.2 test\.minecraft",
                ),
                mods_dir: PathBuf::from(
                    r"C:\AppData\Roaming\PrismLauncher\instances\26.2 test\.minecraft\mods",
                ),
                fabric: FabricStatus::Detected {
                    loader_version: Some("0.16.12".to_string()),
                    mc_versions: vec!["26.2".to_string()],
                    source: "mmc-pack.json".to_string(),
                },
                installed_mods: Vec::new(),
            },
            InstanceInfo {
                display_name: ".minecraft".to_string(),
                launcher: LauncherKind::Official,
                selected_path: PathBuf::from(r"C:\AppData\Roaming\.minecraft"),
                game_dir: PathBuf::from(r"C:\AppData\Roaming\.minecraft"),
                mods_dir: PathBuf::from(r"C:\AppData\Roaming\.minecraft\mods"),
                fabric: FabricStatus::Detected {
                    loader_version: Some("0.16.10".to_string()),
                    mc_versions: vec!["1.21.11".to_string()],
                    source: "versions/".to_string(),
                },
                installed_mods: Vec::new(),
            },
        ]
    }

    #[test]
    fn renders_visual_snapshots_cleanly() {
        let Some(out_dir) = std::env::var_os("MARLOWWW_SNAPSHOT_DIR").map(PathBuf::from) else {
            return;
        };
        std::fs::create_dir_all(&out_dir).unwrap();

        let render_view = |file: &str,
                           step: WizardStep,
                           tour: TourStep,
                           selected: Option<InstanceInfo>,
                           ghost_name: &str,
                           install: InstallState| {
            let releases = sample_releases();
            let instances = sample_instances();
            let ghost_str = ghost_name.to_string();

            let (mut test, _) = TestingRunner::new(
                move || {
                    let releases_state = use_state(|| ReleasesState::Loaded(releases.clone()));
                    let detected_instances = use_state(|| instances.clone());
                    let selected_instance = use_state(|| selected.clone());
                    let selected_release_idx = use_state(|| 0usize);
                    let selected_platform = use_state(|| PlatformKind::Desktop);
                    let selected_asset_name =
                        use_state(|| Some("marlowwwclient-5.5-26.3.jar".to_string()));
                    let replace_existing = use_state(|| true);
                    let custom_jar_name = use_state(|| ghost_str.clone());
                    let wizard_step = use_state(|| step);
                    let tour_step = use_state(|| tour);
                    let instance_search = use_state(String::new);
                    let launcher_filter = use_state(|| LauncherFilter::All);
                    let install_state = use_state(|| install.clone());

                    LauncherShell {
                        releases_state,
                        detected_instances,
                        selected_instance,
                        selected_release_idx,
                        selected_platform,
                        selected_asset_name,
                        replace_existing,
                        custom_jar_name,
                        wizard_step,
                        tour_step,
                        instance_search,
                        launcher_filter,
                        install_state,
                        persist_settings: false,
                    }
                },
                Size2D::new(1120.0, 730.0),
                |_| {},
                1.0,
            );

            test.set_fonts(HashMap::from([
                ("Geist", theme::GEIST_FONT),
                ("Geist Mono", theme::GEIST_MONO_FONT),
            ]));
            test.set_default_fonts(&[Cow::Borrowed("Geist")]);
            test.poll(Duration::from_millis(20), Duration::from_millis(550));
            test.render_to_file(out_dir.join(file));
        };

        let instances = sample_instances();

        render_view(
            "oc_01_versions.png",
            WizardStep::SelectVersion,
            TourStep::Hidden,
            None,
            "",
            InstallState::Idle,
        );

        render_view(
            "oc_02_instances.png",
            WizardStep::SelectInstance,
            TourStep::Hidden,
            Some(instances[0].clone()),
            "Sodium Extra",
            InstallState::Idle,
        );

        render_view(
            "oc_03_stealth_jar.png",
            WizardStep::NameJar,
            TourStep::Hidden,
            Some(instances[0].clone()),
            "Sodium Extra",
            InstallState::Idle,
        );

        render_view(
            "oc_04_spotlight_left.png",
            WizardStep::SelectVersion,
            TourStep::SpotVersion,
            None,
            "",
            InstallState::Idle,
        );

        render_view(
            "oc_05_spotlight_right.png",
            WizardStep::Install,
            TourStep::SpotInstall,
            Some(instances[0].clone()),
            "Sodium Extra",
            InstallState::Idle,
        );

        render_view(
            "oc_06_onboarding_modal.png",
            WizardStep::SelectVersion,
            TourStep::Welcome,
            None,
            "",
            InstallState::Idle,
        );
    }

    #[test]
    fn clicking_update_and_dock_buttons_never_panics() {
        use freya_testing::prelude::*;

        let find_label_center = |test: &TestingRunner, needle: &str| -> (f64, f64) {
            test.find(|node, element| {
                let any_el = element as &dyn std::any::Any;
                if let Some(lbl) = any_el.downcast_ref::<freya::elements::label::LabelElement>() {
                    if lbl.text == needle {
                        let area = node.layout().visible_area();
                        return Some((
                            (area.min_x() + area.width() * 0.5) as f64,
                            (area.min_y() + area.height() * 0.5) as f64,
                        ));
                    }
                }
                None
            })
            .unwrap_or_else(|| panic!("Could not find label {needle}"))
        };

        for mode in 0..5 {
            let releases = sample_releases();
            let mut instances = sample_instances();
            let tmp_dir = std::env::temp_dir().join(format!("marlowww_upd_test_{mode}"));
            let _ = std::fs::remove_dir_all(&tmp_dir);
            std::fs::create_dir_all(tmp_dir.join("mods")).unwrap();
            instances[0].selected_path = tmp_dir.clone();
            instances[0].game_dir = tmp_dir.clone();
            instances[0].mods_dir = tmp_dir.join("mods");
            let first_inst = instances[0].clone();
            let start_step = match mode {
                2 => WizardStep::SelectVersion,
                3 => WizardStep::Install,
                4 => WizardStep::SelectVersion,
                _ => WizardStep::SelectInstance,
            };

            let (mut test, step_handle) = TestingRunner::new(
                move || {
                    let releases_state = use_state(|| ReleasesState::Loaded(releases.clone()));
                    let detected_instances = use_state(|| instances.clone());
                    let selected_instance = use_state(|| Some(first_inst.clone()));
                    let selected_release_idx = use_state(|| 0usize);
                    let selected_platform = use_state(|| PlatformKind::Desktop);
                    let selected_asset_name =
                        use_state(|| Some("marlowwwclient-5.5-26.3.jar".to_string()));
                    let replace_existing = use_state(|| true);
                    let custom_jar_name = use_state(String::new);
                    let wizard_step = use_collapse_step(start_step);
                    let tour_step = use_state(|| TourStep::Hidden);
                    let instance_search = use_state(String::new);
                    let launcher_filter = use_state(|| LauncherFilter::All);
                    let install_state = use_state(|| InstallState::Idle);

                    LauncherShell {
                        releases_state,
                        detected_instances,
                        selected_instance,
                        selected_release_idx,
                        selected_platform,
                        selected_asset_name,
                        replace_existing,
                        custom_jar_name,
                        wizard_step,
                        tour_step,
                        instance_search,
                        launcher_filter,
                        install_state,
                        persist_settings: false,
                    }
                },
                Size2D::new(1120.0, 730.0),
                |runner| runner.provide_root_context(|| State::create(start_step)),
                1.0,
            );

            test.set_fonts(HashMap::from([
                ("Geist", theme::GEIST_FONT),
                ("Geist Mono", theme::GEIST_MONO_FONT),
            ]));
            test.set_default_fonts(&[Cow::Borrowed("Geist")]);
            test.poll(Duration::from_millis(16), Duration::from_millis(90));

            if mode == 4 {
                let c1 = find_label_center(&test, "Continue");
                test.click_cursor(c1);
                assert_eq!(*step_handle.peek(), WizardStep::NameJar);
                let c2 = find_label_center(&test, "Continue");
                test.click_cursor(c2);
                assert_eq!(*step_handle.peek(), WizardStep::SelectInstance);
                let c3 = find_label_center(&test, "Continue");
                test.click_cursor(c3);
                assert_eq!(*step_handle.peek(), WizardStep::Install);
                let install_pt = find_label_center(&test, "Install Client");
                test.click_cursor(install_pt);
            } else {
                let target = if mode == 0 {
                    find_label_center(&test, "Update")
                } else {
                    find_label_center(&test, "Update to Latest")
                };

                test.move_cursor(target);
                test.click_cursor(target);
                test.move_cursor((target.0 + 4.0, target.1 + 4.0));
                assert_eq!(*step_handle.peek(), WizardStep::Install);
            }

            test.poll(Duration::from_millis(16), Duration::from_millis(240));
            let _ = std::fs::remove_dir_all(&tmp_dir);
        }
    }

    fn use_collapse_step(_initial: WizardStep) -> State<WizardStep> {
        use_consume::<State<WizardStep>>()
    }
}
