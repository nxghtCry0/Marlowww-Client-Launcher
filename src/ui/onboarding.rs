use freya::{
    animation::*,
    prelude::*,
};

use crate::{
    models::TourStep,
    theme,
    ui::components::{
        ActionButton,
        BrandLogo,
        ButtonVariant,
    },
};

#[derive(Clone, PartialEq)]
pub struct GuideMascotBadge {
    pub size: f32,
}

impl Component for GuideMascotBadge {
    fn render(&self) -> impl IntoElement {
        let bob = use_animation(|conf| {
            conf.on_creation(OnCreation::Run);
            conf.on_finish(OnFinish::reverse());
            (
                AnimNum::new(-2.5, 2.5)
                    .time(1250)
                    .ease(Ease::InOut)
                    .function(Function::Sine),
                AnimColor::new(theme::ACCENT, theme::ACCENT_GLOW)
                    .time(1250)
                    .ease(Ease::InOut)
                    .function(Function::Sine),
            )
        });
        let (dy, ring_color) = bob.get().value();
        let inner = (self.size - 12.0).max(18.0);

        rect()
            .width(Size::px(self.size))
            .height(Size::px(self.size + 6.0))
            .center()
            .offset_y(dy)
            .child(
                rect()
                    .width(Size::px(self.size))
                    .height(Size::px(self.size))
                    .corner_radius(10.0)
                    .center()
                    .background(theme::ACCENT_SOFT)
                    .border(
                        Border::new()
                            .width(1.5)
                            .fill(ring_color)
                            .alignment(BorderAlignment::Inner),
                    )
                    .child(
                        ImageViewer::new(("marlowww-guide-m", theme::WINDOW_ICON))
                            .width(Size::px(inner))
                            .height(Size::px(inner)),
                    ),
            )
    }
}

#[derive(Clone, PartialEq)]
pub struct SpotlightZoneWrapper {
    pub focused: bool,
    pub dimmed: bool,
    pub pad_inner: f32,
    pub width: Size,
    pub height: Size,
    pub radius: f32,
    pub child: Element,
}

impl SpotlightZoneWrapper {
    pub fn new(
        focused: bool,
        dimmed: bool,
        pad_inner: f32,
        width: Size,
        height: Size,
        radius: f32,
        child: impl IntoElement,
    ) -> Self {
        Self {
            focused,
            dimmed,
            pad_inner,
            width,
            height,
            radius,
            child: child.into_element(),
        }
    }
}

impl Component for SpotlightZoneWrapper {
    fn render(&self) -> impl IntoElement {
        let target_opacity = if self.dimmed { 0.22 } else { 1.0 };
        let opacity_state = use_reactive(&target_opacity);
        let opacity_anim = use_animation_transition(opacity_state, |from: f32, to| {
            AnimNum::new(from, to)
                .time(240)
                .ease(Ease::Out)
                .function(Function::Expo)
        });

        let pulse = use_animation_with_dependencies(&self.focused, |conf, &focused| {
            if focused {
                conf.on_creation(OnCreation::Run);
                conf.on_change(OnChange::Rerun);
                conf.on_finish(OnFinish::reverse());
                (
                    AnimColor::new(theme::ACCENT, theme::ACCENT_GLOW)
                        .time(1150)
                        .ease(Ease::InOut)
                        .function(Function::Sine),
                    AnimNum::new(0.24, 0.48)
                        .time(1150)
                        .ease(Ease::InOut)
                        .function(Function::Sine),
                )
            } else {
                conf.on_creation(OnCreation::Finish);
                conf.on_change(OnChange::Finish);
                (
                    AnimColor::new(Color::TRANSPARENT, Color::TRANSPARENT).time(1),
                    AnimNum::new(0.0, 0.0).time(1),
                )
            }
        });

        let (ring_color, glow_alpha) = pulse.get().value();

        if !self.focused {
            return rect()
                .width(self.width.clone())
                .height(self.height.clone())
                .opacity(opacity_anim.get().value())
                .child(self.child.clone());
        }

        rect()
            .width(self.width.clone())
            .height(self.height.clone())
            .corner_radius(self.radius + 2.0)
            .padding(Gaps::new_all(self.pad_inner))
            .background(if self.pad_inner > 4.0 {
                Color::from_argb(120, 16, 20, 27)
            } else {
                Color::TRANSPARENT
            })
            .shadow(
                Shadow::new()
                    .y(0.0)
                    .blur(26.0)
                    .color(theme::ACCENT.with_af32(glow_alpha)),
            )
            .border(
                Border::new()
                    .width(1.5)
                    .fill(ring_color)
                    .alignment(BorderAlignment::Inner),
            )
            .child(self.child.clone())
    }
}

#[derive(Clone, PartialEq)]
pub struct SpotlightCoachBar {
    pub step: TourStep,
    pub on_prev: NoArgCallback<()>,
    pub on_next: NoArgCallback<()>,
    pub on_skip: NoArgCallback<()>,
}

impl Component for SpotlightCoachBar {
    fn render(&self) -> impl IntoElement {
        let anim = use_animation_with_dependencies(&self.step, |conf, _| {
            conf.on_creation(OnCreation::Run);
            conf.on_change(OnChange::Rerun);
            (
                AnimNum::new(0.0, 1.0)
                    .time(220)
                    .ease(Ease::Out)
                    .function(Function::Expo),
                AnimNum::new(-8.0, 0.0)
                    .time(220)
                    .ease(Ease::Out)
                    .function(Function::Expo),
            )
        });
        let (opacity, slide_y) = anim.get().value();

        let on_prev = self.on_prev.clone();
        let on_next = self.on_next.clone();
        let on_skip = self.on_skip.clone();
        let is_last = self.step == TourStep::SpotComplete;

        rect()
            .width(Size::fill())
            .opacity(opacity)
            .offset_y(slide_y)
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .spacing(14.0)
                    .padding(Gaps::new(11.0, 16.0, 11.0, 14.0))
                    .corner_radius(theme::RADIUS_CARD)
                    .background(theme::BG_PANEL)
                    .border(
                        Border::new()
                            .width(1.0)
                            .fill(theme::ACCENT_RING)
                            .alignment(BorderAlignment::Inner),
                    )
                    .shadow(Shadow::new().y(8.0).blur(24.0).color((0, 0, 0, 130)))
                    .child(GuideMascotBadge { size: 38.0 })
                    .child(
                        rect()
                            .width(Size::flex(1.0))
                            .vertical()
                            .spacing(3.0)
                            .child(
                                rect()
                                    .horizontal()
                                    .cross_align(Alignment::Center)
                                    .spacing(8.0)
                                    .child(
                                        rect()
                                            .padding(Gaps::new(2.0, 8.0, 2.0, 8.0))
                                            .corner_radius(6.0)
                                            .background(theme::ACCENT_SOFT)
                                            .child(
                                                label()
                                                    .text(format!(
                                                        "Guide {} of {}",
                                                        self.step.step_index(),
                                                        TourStep::TOTAL_STEPS
                                                    ))
                                                    .font_family("Geist Mono")
                                                    .font_size(10.5)
                                                    .font_weight(FontWeight::SEMI_BOLD)
                                                    .color(theme::ACCENT_GLOW),
                                            ),
                                    )
                                    .child(
                                        label()
                                            .text(self.step.title())
                                            .color(theme::TEXT_PRIMARY)
                                            .font_size(13.5)
                                            .font_weight(FontWeight::SEMI_BOLD)
                                            .max_lines(1),
                                    ),
                            )
                            .child(
                                label()
                                    .width(Size::fill())
                                    .text(self.step.body())
                                    .color(theme::TEXT_SECONDARY)
                                    .font_size(12.0)
                                    .max_lines(2),
                            ),
                    )
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(8.0)
                            .child(
                                ActionButton::new("Skip", move || on_skip.call())
                                    .variant(ButtonVariant::Subtle)
                                    .compact(true),
                            )
                            .child(
                                ActionButton::new("Back", move || on_prev.call())
                                    .icon(theme::PHOSPHOR_ARROW_LEFT)
                                    .variant(ButtonVariant::Control)
                                    .compact(true),
                            )
                            .child(
                                ActionButton::new(if is_last { "Done" } else { "Next" }, move || {
                                    on_next.call()
                                })
                                .trailing_icon(if is_last {
                                    theme::PHOSPHOR_CHECK
                                } else {
                                    theme::PHOSPHOR_ARROW_RIGHT
                                })
                                .variant(ButtonVariant::Primary)
                                .compact(true),
                            ),
                    ),
            )
    }
}

#[derive(Clone, PartialEq)]
pub struct OnboardingModalOverlay {
    pub step: TourStep,
    pub on_prev: NoArgCallback<()>,
    pub on_next: NoArgCallback<()>,
    pub on_skip: NoArgCallback<()>,
}

impl Component for OnboardingModalOverlay {
    fn render(&self) -> impl IntoElement {
        if !self.step.is_modal_intro() {
            return rect().width(Size::px(0.0)).height(Size::px(0.0));
        }

        let card_anim = use_animation_with_dependencies(&self.step, |conf, _| {
            conf.on_creation(OnCreation::Run);
            conf.on_change(OnChange::Rerun);
            (
                AnimNum::new(0.96, 1.0)
                    .time(190)
                    .ease(Ease::Out)
                    .function(Function::Expo),
                AnimNum::new(10.0, 0.0)
                    .time(190)
                    .ease(Ease::Out)
                    .function(Function::Expo),
            )
        });
        let (scale_v, slide_y) = card_anim.get().value();

        let on_prev = self.on_prev.clone();
        let on_next = self.on_next.clone();
        let on_skip = self.on_skip.clone();
        let can_go_back = self.step != TourStep::Welcome;
        let cur_idx = self.step.step_index();
        let is_fabric_step = self.step == TourStep::FabricInstall;

        let (banner_id, banner_bytes) = match self.step {
            TourStep::Welcome => ("intro-banner-wilderness", theme::BANNER_WILDERNESS),
            TourStep::WhatIsThis => ("intro-banner-sky", theme::BANNER_SKY_ARENA),
            _ => ("intro-banner-vault", theme::BANNER_TRIAL_VAULT),
        };

        rect()
            .position(Position::new_global().top(0.0).left(0.0))
            .width(Size::window_percent(100.0))
            .height(Size::window_percent(100.0))
            .layer(Layer::Overlay)
            .background(theme::BG_TOPBAR.with_af32(0.90))
            .center()
            .child(
                rect()
                    .offset_y(slide_y)
                    .child(
                        rect()
                            .width(Size::px(540.0))
                            .scale(scale_v)
                            .corner_radius(theme::RADIUS_PANEL)
                            .overflow(Overflow::Clip)
                            .background(theme::BG_PANEL)
                            .shadow(Shadow::new().y(18.0).blur(42.0).color((0, 0, 0, 190)))
                            .border(
                                Border::new()
                                    .width(1.0)
                                    .fill(theme::BORDER_CONTROL)
                                    .alignment(BorderAlignment::Inner),
                            )
                            .vertical()
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .height(Size::px(124.0))
                                    .overflow(Overflow::Clip)
                                    .child(
                                        ImageViewer::new((banner_id, banner_bytes))
                                            .width(Size::fill())
                                            .height(Size::fill())
                                            .aspect_ratio(AspectRatio::Max)
                                            .image_cover(ImageCover::Center),
                                    )
                                    .child(
                                        rect()
                                            .position(Position::new_absolute().top(0.0).left(0.0))
                                            .width(Size::percent(100.0))
                                            .height(Size::percent(100.0))
                                            .horizontal()
                                            .main_align(Alignment::SpaceBetween)
                                            .cross_align(Alignment::End)
                                            .padding(Gaps::new(16.0, 22.0, 14.0, 22.0))
                                            .child(BrandLogo::new(168.0, 39.5))
                                            .child(
                                                rect()
                                                    .padding(Gaps::new(4.0, 10.0, 4.0, 10.0))
                                                    .corner_radius(6.0)
                                                    .background(theme::BG_PILL_SCRIM)
                                                    .border(
                                                        Border::new()
                                                            .width(1.0)
                                                            .fill(Color::from_argb(26, 255, 255, 255)),
                                                    )
                                                    .child(
                                                        label()
                                                            .text(format!(
                                                                "Step {} of {}",
                                                                cur_idx,
                                                                TourStep::TOTAL_STEPS
                                                            ))
                                                            .font_family("Geist Mono")
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
                                    .vertical()
                                    .spacing(16.0)
                                    .padding(Gaps::new(20.0, 24.0, 22.0, 24.0))
                                    .child(
                                        rect()
                                            .width(Size::fill())
                                            .vertical()
                                            .spacing(7.0)
                                            .child(
                                                label()
                                                    .text(self.step.title())
                                                    .color(theme::TEXT_PRIMARY)
                                                    .font_size(20.0)
                                                    .font_weight(FontWeight::BOLD)
                                                    .letter_spacing(-0.4),
                                            )
                                            .child(
                                                label()
                                                    .width(Size::fill())
                                                    .text(self.step.body())
                                                    .color(theme::TEXT_SECONDARY)
                                                    .font_size(13.5),
                                            ),
                                    )
                                    .maybe_child(is_fabric_step.then(|| {
                                        rect()
                                            .width(Size::fill())
                                            .vertical()
                                            .spacing(9.0)
                                            .padding(Gaps::new(12.0, 14.0, 12.0, 14.0))
                                            .corner_radius(theme::RADIUS_CONTROL)
                                            .background(theme::BG_CARD)
                                            .border(
                                                Border::new()
                                                    .width(1.0)
                                                    .fill(theme::BORDER_SUBTLE)
                                                    .alignment(BorderAlignment::Inner),
                                            )
                                            .child(
                                                rect()
                                                    .width(Size::fill())
                                                    .horizontal()
                                                    .main_align(Alignment::SpaceBetween)
                                                    .cross_align(Alignment::Center)
                                                    .child(
                                                        rect()
                                                            .vertical()
                                                            .spacing(2.0)
                                                            .child(
                                                                label()
                                                                    .text("Fabric Loader Installer")
                                                                    .color(theme::TEXT_PRIMARY)
                                                                    .font_size(13.0)
                                                                    .font_weight(FontWeight::SEMI_BOLD),
                                                            )
                                                            .child(
                                                                label()
                                                                    .text("fabricmc.net/use/installer")
                                                                    .font_family("Geist Mono")
                                                                    .color(theme::TEXT_MUTED)
                                                                    .font_size(11.5),
                                                            ),
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
                                                    ),
                                            )
                                            .child(
                                                label()
                                                    .text("OneClient, Modrinth, and Prism already manage Fabric per instance.")
                                                    .color(theme::TEXT_SECONDARY)
                                                    .font_size(12.0),
                                            )
                                    }))
                                    .child(
                                        rect()
                                            .width(Size::fill())
                                            .horizontal()
                                            .main_align(Alignment::SpaceBetween)
                                            .cross_align(Alignment::Center)
                                            .padding(Gaps::new(4.0, 0.0, 0.0, 0.0))
                                            .child(
                                                rect()
                                                    .horizontal()
                                                    .cross_align(Alignment::Center)
                                                    .spacing(5.0)
                                                    .children(
                                                        (1..=TourStep::TOTAL_STEPS).map(|idx| {
                                                            let is_cur = idx == cur_idx;
                                                            rect()
                                                                .width(Size::px(if is_cur {
                                                                    20.0
                                                                } else {
                                                                    6.0
                                                                }))
                                                                .height(Size::px(5.0))
                                                                .rounded_full()
                                                                .background(if is_cur {
                                                                    theme::ACCENT
                                                                } else if idx < cur_idx {
                                                                    theme::ACCENT_RING
                                                                } else {
                                                                    theme::BORDER_CONTROL
                                                                })
                                                        }),
                                                    ),
                                            )
                                            .child(
                                                rect()
                                                    .horizontal()
                                                    .cross_align(Alignment::Center)
                                                    .spacing(8.0)
                                                    .child(
                                                        ActionButton::new("Skip", move || {
                                                            on_skip.call()
                                                        })
                                                        .variant(ButtonVariant::Subtle),
                                                    )
                                                    .maybe_child(can_go_back.then(|| {
                                                        ActionButton::new("Back", move || {
                                                            on_prev.call()
                                                        })
                                                        .icon(theme::PHOSPHOR_ARROW_LEFT)
                                                        .variant(ButtonVariant::Control)
                                                    }))
                                                    .child(
                                                        ActionButton::new(
                                                            if is_fabric_step {
                                                                "Start Walkthrough"
                                                            } else {
                                                                "Next"
                                                            },
                                                            move || on_next.call(),
                                                        )
                                                        .trailing_icon(theme::PHOSPHOR_ARROW_RIGHT)
                                                        .variant(ButtonVariant::Primary),
                                                    ),
                                            ),
                                    ),
                            ),
                    ),
            )
    }
}
