use freya::{animation::*, prelude::*};

use crate::{models::WizardStep, theme};

fn try_set_bool(state: &mut State<bool>, value: bool) {
    if let Some(mut guard) = state.try_write() {
        *guard = value;
    }
}

#[derive(Clone, PartialEq)]
pub struct PhosphorIcon {
    pub bytes: &'static [u8],
    pub size: f32,
    pub color: Color,
}

impl PhosphorIcon {
    pub fn new(bytes: &'static [u8], size: f32, color: Color) -> Self {
        Self { bytes, size, color }
    }
}

impl Component for PhosphorIcon {
    fn render(&self) -> impl IntoElement {
        SvgViewer::new(self.bytes)
            .show_loader(false)
            .width(Size::px(self.size))
            .height(Size::px(self.size))
            .color(self.color)
            .fill(self.color)
    }
}

#[derive(Clone, PartialEq)]
pub struct BrandLogo {
    pub width: f32,
    pub height: f32,
}

impl BrandLogo {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

impl Component for BrandLogo {
    fn render(&self) -> impl IntoElement {
        ImageViewer::new(("marlowww-official-logo", theme::OFFICIAL_LOGO))
            .width(Size::px(self.width))
            .height(Size::px(self.height))
    }
}

#[derive(Clone, PartialEq)]
pub struct MotionTransition {
    pub trigger_key: String,
    pub slide_x: f32,
    pub slide_y: f32,
    pub duration_ms: u64,
    pub width: Size,
    pub height: Size,
    pub child: Element,
}

impl MotionTransition {
    pub fn new(trigger_key: impl Into<String>, child: impl IntoElement) -> Self {
        Self {
            trigger_key: trigger_key.into(),
            slide_x: 0.0,
            slide_y: 8.0,
            duration_ms: 220,
            width: Size::fill(),
            height: Size::fill(),
            child: child.into_element(),
        }
    }

    pub fn slide(mut self, x: f32, y: f32) -> Self {
        self.slide_x = x;
        self.slide_y = y;
        self
    }

    pub fn height(mut self, height: impl Into<Size>) -> Self {
        self.height = height.into();
        self
    }
}

impl Component for MotionTransition {
    fn render(&self) -> impl IntoElement {
        let sx = self.slide_x;
        let sy = self.slide_y;
        let dur = self.duration_ms;
        let anim = use_animation_with_dependencies(&self.trigger_key, move |conf, _| {
            conf.on_creation(OnCreation::Run);
            conf.on_change(OnChange::Rerun);
            (
                AnimNum::new(0.0, 1.0)
                    .time(dur)
                    .ease(Ease::Out)
                    .function(Function::Expo),
                AnimNum::new(sx, 0.0)
                    .time(dur)
                    .ease(Ease::Out)
                    .function(Function::Expo),
                AnimNum::new(sy, 0.0)
                    .time(dur)
                    .ease(Ease::Out)
                    .function(Function::Expo),
            )
        });
        let (opacity, dx, dy) = anim.get().value();

        rect()
            .width(self.width.clone())
            .height(self.height.clone())
            .opacity(opacity)
            .offset_x(dx)
            .offset_y(dy)
            .child(self.child.clone())
    }
}

#[derive(Clone, PartialEq)]
pub struct TopNavTab {
    pub step: WizardStep,
    pub active: bool,
    pub completed: bool,
    pub on_select: NoArgCallback<()>,
    pub key: DiffKey,
}

impl KeyExt for TopNavTab {
    fn key_mut(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for TopNavTab {
    fn render(&self) -> impl IntoElement {
        let mut hovered = use_state(|| false);
        let mut pressed = use_state(|| false);

        let target_bg = if self.active {
            theme::BG_CONTROL
        } else if hovered() {
            Color::from_rgb(18, 22, 28)
        } else {
            Color::TRANSPARENT
        };
        let target_border = if self.active {
            theme::BORDER_CONTROL
        } else {
            Color::TRANSPARENT
        };

        let bg_state = use_reactive(&target_bg);
        let border_state = use_reactive(&target_border);
        let bg_anim = use_animation_transition(bg_state, |from: Color, to| {
            AnimColor::new(from, to)
                .time(160)
                .ease(Ease::Out)
                .function(Function::Expo)
        });
        let border_anim = use_animation_transition(border_state, |from: Color, to| {
            AnimColor::new(from, to)
                .time(160)
                .ease(Ease::Out)
                .function(Function::Expo)
        });

        let scale_target = if pressed() { 0.965 } else { 1.0 };
        let scale_state = use_reactive(&scale_target);
        let scale_anim = use_animation_transition(scale_state, |from: f32, to| {
            AnimNum::new(from, to)
                .time(110)
                .ease(Ease::Out)
                .function(Function::Cubic)
        });

        let on_select = self.on_select.clone();

        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(7.0)
            .padding(Gaps::new(7.0, 15.0, 7.0, 15.0))
            .corner_radius(theme::RADIUS_CONTROL)
            .background(bg_anim.get().value())
            .border(
                Border::new()
                    .width(1.0)
                    .fill(border_anim.get().value())
                    .alignment(BorderAlignment::Inner),
            )
            .scale(scale_anim.get().value())
            .cursor(CursorIcon::Pointer)
            .on_pointer_enter(move |_| try_set_bool(&mut hovered, true))
            .on_pointer_leave(move |_| {
                try_set_bool(&mut hovered, false);
                try_set_bool(&mut pressed, false);
            })
            .on_pointer_down(move |_| try_set_bool(&mut pressed, true))
            .on_press(move |_| {
                try_set_bool(&mut pressed, false);
                on_select.call();
            })
            .maybe_child((self.completed && !self.active).then(|| {
                rect()
                    .width(Size::px(5.5))
                    .height(Size::px(5.5))
                    .rounded_full()
                    .background(theme::ACCENT_RING)
            }))
            .child(
                label()
                    .text(self.step.short_label())
                    .color(if self.active || hovered() {
                        theme::TEXT_PRIMARY
                    } else {
                        theme::TEXT_SECONDARY
                    })
                    .font_size(13.0)
                    .font_weight(if self.active {
                        FontWeight::SEMI_BOLD
                    } else {
                        FontWeight::MEDIUM
                    })
                    .max_lines(1),
            )
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}

#[derive(Clone, PartialEq)]
pub struct UnderlineTab {
    pub label: String,
    pub selected: bool,
    pub on_press: NoArgCallback<()>,
    pub key: DiffKey,
}

impl KeyExt for UnderlineTab {
    fn key_mut(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for UnderlineTab {
    fn render(&self) -> impl IntoElement {
        let mut hovered = use_state(|| false);

        let line_target = if self.selected {
            theme::TEXT_PRIMARY
        } else if hovered() {
            theme::BORDER_HOVER
        } else {
            Color::TRANSPARENT
        };
        let line_state = use_reactive(&line_target);
        let line_anim = use_animation_transition(line_state, |from: Color, to| {
            AnimColor::new(from, to)
                .time(170)
                .ease(Ease::Out)
                .function(Function::Expo)
        });

        let fg_target = if self.selected {
            theme::TEXT_PRIMARY
        } else if hovered() {
            Color::from_rgb(206, 212, 224)
        } else {
            theme::TEXT_MUTED
        };
        let fg_state = use_reactive(&fg_target);
        let fg_anim = use_animation_transition(fg_state, |from: Color, to| {
            AnimColor::new(from, to)
                .time(150)
                .ease(Ease::Out)
                .function(Function::Cubic)
        });

        let on_press = self.on_press.clone();

        rect()
            .padding(Gaps::new(4.0, 2.0, 9.0, 2.0))
            .border(
                Border::new()
                    .width(BorderWidth {
                        top: 0.0,
                        right: 0.0,
                        bottom: 2.0,
                        left: 0.0,
                    })
                    .fill(line_anim.get().value()),
            )
            .cursor(CursorIcon::Pointer)
            .on_pointer_enter(move |_| try_set_bool(&mut hovered, true))
            .on_pointer_leave(move |_| try_set_bool(&mut hovered, false))
            .on_press(move |_| on_press.call())
            .child(
                label()
                    .text(self.label.clone())
                    .color(fg_anim.get().value())
                    .font_size(13.0)
                    .font_weight(if self.selected {
                        FontWeight::SEMI_BOLD
                    } else {
                        FontWeight::MEDIUM
                    })
                    .max_lines(1),
            )
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}

#[derive(Clone, PartialEq)]
pub struct RefreshButton {
    pub spinning: bool,
    pub on_press: NoArgCallback<()>,
}

impl Component for RefreshButton {
    fn render(&self) -> impl IntoElement {
        let mut hovered = use_state(|| false);
        let mut pressed = use_state(|| false);

        let spin = use_animation_with_dependencies(&self.spinning, |conf, &spinning| {
            if spinning {
                conf.on_creation(OnCreation::Run);
                conf.on_change(OnChange::Rerun);
                conf.on_finish(OnFinish::restart());
                AnimNum::new(0.0, 360.0)
                    .time(720)
                    .ease(Ease::InOut)
                    .function(Function::Linear)
            } else {
                conf.on_creation(OnCreation::Finish);
                conf.on_change(OnChange::Finish);
                AnimNum::new(0.0, 0.0).time(1)
            }
        });

        let target_bg = if hovered() {
            theme::BG_HOVER
        } else {
            theme::BG_CONTROL
        };
        let bg_state = use_reactive(&target_bg);
        let bg_anim = use_animation_transition(bg_state, |from: Color, to| {
            AnimColor::new(from, to)
                .time(150)
                .ease(Ease::Out)
                .function(Function::Cubic)
        });

        let scale_target = if pressed() { 0.92 } else { 1.0 };
        let scale_state = use_reactive(&scale_target);
        let scale_anim = use_animation_transition(scale_state, |from: f32, to| {
            AnimNum::new(from, to)
                .time(110)
                .ease(Ease::Out)
                .function(Function::Cubic)
        });

        let icon_color = if hovered() || self.spinning {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_SECONDARY
        };
        let on_press = self.on_press.clone();
        let spinning = self.spinning;

        rect()
            .width(Size::px(34.0))
            .height(Size::px(34.0))
            .corner_radius(theme::RADIUS_CONTROL)
            .center()
            .background(bg_anim.get().value())
            .border(
                Border::new()
                    .width(1.0)
                    .fill(if hovered() {
                        theme::BORDER_HOVER
                    } else {
                        theme::BORDER_CONTROL
                    })
                    .alignment(BorderAlignment::Inner),
            )
            .scale(scale_anim.get().value())
            .cursor(if spinning {
                CursorIcon::Default
            } else {
                CursorIcon::Pointer
            })
            .on_pointer_enter(move |_| try_set_bool(&mut hovered, true))
            .on_pointer_leave(move |_| {
                try_set_bool(&mut hovered, false);
                try_set_bool(&mut pressed, false);
            })
            .on_pointer_down(move |_| {
                if !spinning {
                    try_set_bool(&mut pressed, true);
                }
            })
            .on_press(move |_| {
                try_set_bool(&mut pressed, false);
                if !spinning {
                    on_press.call();
                }
            })
            .child(
                SvgViewer::new(theme::PHOSPHOR_REFRESH)
                    .show_loader(false)
                    .width(Size::px(15.0))
                    .height(Size::px(15.0))
                    .color(icon_color)
                    .fill(icon_color)
                    .rotation(spin.get().value()),
            )
    }
}

#[derive(Clone, PartialEq)]
pub struct IconSquareButton {
    pub icon: &'static [u8],
    pub disabled: bool,
    pub on_press: NoArgCallback<()>,
}

impl Component for IconSquareButton {
    fn render(&self) -> impl IntoElement {
        let mut hovered = use_state(|| false);
        let mut pressed = use_state(|| false);
        let interactive = !self.disabled;

        let target_bg = if !interactive {
            theme::BG_CARD
        } else if hovered() {
            theme::BG_HOVER
        } else {
            theme::BG_CONTROL
        };
        let bg_state = use_reactive(&target_bg);
        let bg_anim = use_animation_transition(bg_state, |from: Color, to| {
            AnimColor::new(from, to)
                .time(140)
                .ease(Ease::Out)
                .function(Function::Cubic)
        });

        let scale_target = if pressed() && interactive { 0.93 } else { 1.0 };
        let scale_state = use_reactive(&scale_target);
        let scale_anim = use_animation_transition(scale_state, |from: f32, to| {
            AnimNum::new(from, to)
                .time(110)
                .ease(Ease::Out)
                .function(Function::Cubic)
        });

        let fg = if !interactive {
            theme::TEXT_DISABLED
        } else if hovered() {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_SECONDARY
        };
        let on_press = self.on_press.clone();

        rect()
            .width(Size::px(38.0))
            .height(Size::px(38.0))
            .corner_radius(theme::RADIUS_CONTROL)
            .center()
            .background(bg_anim.get().value())
            .border(
                Border::new()
                    .width(1.0)
                    .fill(if !interactive {
                        theme::BORDER_SUBTLE
                    } else if hovered() {
                        theme::BORDER_HOVER
                    } else {
                        theme::BORDER_CONTROL
                    })
                    .alignment(BorderAlignment::Inner),
            )
            .scale(scale_anim.get().value())
            .cursor(if interactive {
                CursorIcon::Pointer
            } else {
                CursorIcon::NotAllowed
            })
            .on_pointer_enter(move |_| try_set_bool(&mut hovered, true))
            .on_pointer_leave(move |_| {
                try_set_bool(&mut hovered, false);
                try_set_bool(&mut pressed, false);
            })
            .on_pointer_down(move |_| {
                if interactive {
                    try_set_bool(&mut pressed, true);
                }
            })
            .on_press(move |_| {
                try_set_bool(&mut pressed, false);
                if interactive {
                    on_press.call();
                }
            })
            .child(PhosphorIcon::new(self.icon, 16.0, fg))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    SecondaryTint,
    Control,
    Subtle,
}

#[derive(Clone, PartialEq)]
pub struct ActionButton {
    pub label: String,
    pub action_tag: &'static str,
    pub icon: Option<&'static [u8]>,
    pub icon_trailing: bool,
    pub variant: ButtonVariant,
    pub compact: bool,
    pub fill_width: bool,
    pub flex_width: bool,
    pub disabled: bool,
    pub on_press: NoArgCallback<()>,
    pub key: DiffKey,
}

impl KeyExt for ActionButton {
    fn key_mut(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl ActionButton {
    pub fn new(label: impl Into<String>, on_press: impl FnMut() + 'static) -> Self {
        Self {
            label: label.into(),
            action_tag: "",
            icon: None,
            icon_trailing: false,
            variant: ButtonVariant::Control,
            compact: false,
            fill_width: false,
            flex_width: false,
            disabled: false,
            on_press: NoArgCallback::new(on_press),
            key: DiffKey::None,
        }
    }

    pub fn action_tag(mut self, tag: &'static str) -> Self {
        self.action_tag = tag;
        self
    }

    pub fn icon(mut self, icon: &'static [u8]) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn trailing_icon(mut self, icon: &'static [u8]) -> Self {
        self.icon = Some(icon);
        self.icon_trailing = true;
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    pub fn fill_width(mut self, fill_width: bool) -> Self {
        self.fill_width = fill_width;
        self
    }

    pub fn flex_width(mut self, flex_width: bool) -> Self {
        self.flex_width = flex_width;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Component for ActionButton {
    fn render(&self) -> impl IntoElement {
        let mut hovered = use_state(|| false);
        let mut pressed = use_state(|| false);
        let interactive = !self.disabled;

        let scale_target = if !interactive {
            1.0
        } else if pressed() {
            0.975
        } else {
            1.0
        };
        let scale_state = use_reactive(&scale_target);
        let scale_anim = use_animation_transition(scale_state, |from: f32, to| {
            AnimNum::new(from, to)
                .time(115)
                .ease(Ease::Out)
                .function(Function::Cubic)
        });

        let lift_target = if interactive && hovered() && !pressed() {
            -1.0
        } else {
            0.0
        };
        let lift_state = use_reactive(&lift_target);
        let lift_anim = use_animation_transition(lift_state, |from: f32, to| {
            AnimNum::new(from, to)
                .time(150)
                .ease(Ease::Out)
                .function(Function::Expo)
        });

        let (base_bg, hover_bg, base_border, hover_border, text_color, hover_text) =
            match self.variant {
                ButtonVariant::Primary => (
                    theme::ACCENT,
                    theme::ACCENT_HOVER,
                    theme::ACCENT,
                    theme::ACCENT_HOVER,
                    Color::WHITE,
                    Color::WHITE,
                ),
                ButtonVariant::SecondaryTint => (
                    theme::ACCENT_SOFT,
                    Color::from_rgb(28, 42, 86),
                    Color::from_rgb(46, 68, 146),
                    theme::ACCENT_RING,
                    Color::from_rgb(196, 210, 255),
                    Color::WHITE,
                ),
                ButtonVariant::Control => (
                    theme::BG_CONTROL,
                    theme::BG_HOVER,
                    theme::BORDER_CONTROL,
                    theme::BORDER_HOVER,
                    theme::TEXT_PRIMARY,
                    theme::TEXT_PRIMARY,
                ),
                ButtonVariant::Subtle => (
                    Color::TRANSPARENT,
                    theme::BG_CONTROL,
                    Color::TRANSPARENT,
                    theme::BORDER_SUBTLE,
                    theme::TEXT_SECONDARY,
                    theme::TEXT_PRIMARY,
                ),
            };

        let target_bg = if !interactive {
            theme::BG_CARD
        } else if hovered() {
            hover_bg
        } else {
            base_bg
        };
        let target_border = if !interactive {
            theme::BORDER_SUBTLE
        } else if hovered() {
            hover_border
        } else {
            base_border
        };

        let bg_state = use_reactive(&target_bg);
        let border_state = use_reactive(&target_border);
        let bg_anim = use_animation_transition(bg_state, |from: Color, to| {
            AnimColor::new(from, to)
                .time(150)
                .ease(Ease::Out)
                .function(Function::Cubic)
        });
        let border_anim = use_animation_transition(border_state, |from: Color, to| {
            AnimColor::new(from, to)
                .time(150)
                .ease(Ease::Out)
                .function(Function::Cubic)
        });

        let fg = if !interactive {
            theme::TEXT_DISABLED
        } else if hovered() {
            hover_text
        } else {
            text_color
        };

        let padding = if self.compact {
            Gaps::new(5.5, 11.0, 5.5, 11.0)
        } else if self.fill_width || self.flex_width {
            Gaps::new(10.0, 16.0, 10.0, 16.0)
        } else {
            match self.variant {
                ButtonVariant::Primary | ButtonVariant::SecondaryTint => {
                    Gaps::new(8.5, 16.0, 8.5, 16.0)
                }
                ButtonVariant::Control => Gaps::new(8.0, 13.0, 8.0, 13.0),
                ButtonVariant::Subtle => Gaps::new(6.0, 10.0, 6.0, 10.0),
            }
        };

        let on_press = self.on_press.clone();
        let font_size = if self.compact { 12.0 } else { 13.0 };
        let outer_w = if self.flex_width {
            Size::flex(1.0)
        } else if self.fill_width {
            Size::fill()
        } else {
            Size::Inner
        };
        let inner_w = if self.fill_width || self.flex_width {
            Size::fill()
        } else {
            Size::Inner
        };

        rect()
            .width(outer_w)
            .offset_y(lift_anim.get().value())
            .child(
                rect()
                    .width(inner_w)
                    .horizontal()
                    .center()
                    .spacing(6.5)
                    .padding(padding)
                    .corner_radius(theme::RADIUS_CONTROL)
                    .background(bg_anim.get().value())
                    .border(
                        Border::new()
                            .width(1.0)
                            .fill(border_anim.get().value())
                            .alignment(BorderAlignment::Inner),
                    )
                    .scale(scale_anim.get().value())
                    .cursor(if interactive {
                        CursorIcon::Pointer
                    } else {
                        CursorIcon::NotAllowed
                    })
                    .on_pointer_enter(move |_| try_set_bool(&mut hovered, true))
                    .on_pointer_leave(move |_| {
                        try_set_bool(&mut hovered, false);
                        try_set_bool(&mut pressed, false);
                    })
                    .on_pointer_down(move |e: Event<PointerEventData>| {
                        e.stop_propagation();
                        if interactive {
                            try_set_bool(&mut pressed, true);
                        }
                    })
                    .on_press(move |e: Event<PressEventData>| {
                        e.stop_propagation();
                        try_set_bool(&mut pressed, false);
                        if interactive {
                            on_press.call();
                        }
                    })
                    .maybe_child(
                        (!self.icon_trailing)
                            .then_some(self.icon)
                            .flatten()
                            .map(|ic| {
                                PhosphorIcon::new(ic, if self.compact { 12.5 } else { 14.0 }, fg)
                            }),
                    )
                    .child(
                        label()
                            .text(self.label.clone())
                            .color(fg)
                            .font_size(font_size)
                            .font_weight(match self.variant {
                                ButtonVariant::Primary | ButtonVariant::SecondaryTint => {
                                    FontWeight::SEMI_BOLD
                                }
                                _ => FontWeight::MEDIUM,
                            })
                            .max_lines(1),
                    )
                    .maybe_child(self.icon_trailing.then_some(self.icon).flatten().map(|ic| {
                        PhosphorIcon::new(ic, if self.compact { 12.0 } else { 13.5 }, fg)
                    })),
            )
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}

#[derive(Clone, PartialEq)]
pub struct ArtworkCard {
    pub banner_id: &'static str,
    pub banner_bytes: &'static [u8],
    pub top_badge: Option<String>,
    pub title: String,
    pub subtitle: String,
    pub subtitle_warn: bool,
    pub selected: bool,
    pub quick_action_label: Option<&'static str>,
    pub anim_group: String,
    pub index: usize,
    pub on_select: NoArgCallback<()>,
    pub on_quick_action: Option<NoArgCallback<()>>,
    pub key: DiffKey,
}

impl KeyExt for ArtworkCard {
    fn key_mut(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for ArtworkCard {
    fn render(&self) -> impl IntoElement {
        let mut hovered = use_state(|| false);
        let mut pressed = use_state(|| false);

        let dur = 210 + (self.index.min(8) as u64) * 26;
        let entrance = use_animation_with_dependencies(&self.anim_group, move |conf, _| {
            conf.on_creation(OnCreation::Run);
            conf.on_change(OnChange::Rerun);
            (
                AnimNum::new(0.0, 1.0)
                    .time(dur)
                    .ease(Ease::Out)
                    .function(Function::Expo),
                AnimNum::new(10.0, 0.0)
                    .time(dur)
                    .ease(Ease::Out)
                    .function(Function::Expo),
            )
        });
        let (enter_opacity, enter_y) = entrance.get().value();

        let lift_target = if hovered() && !pressed() { -3.5 } else { 0.0 };
        let lift_state = use_reactive(&lift_target);
        let lift_anim = use_animation_transition(lift_state, |from: f32, to| {
            AnimNum::new(from, to)
                .time(180)
                .ease(Ease::Out)
                .function(Function::Expo)
        });

        let scale_target = if pressed() {
            0.98
        } else if hovered() {
            1.012
        } else {
            1.0
        };
        let scale_state = use_reactive(&scale_target);
        let scale_anim = use_animation_transition(scale_state, |from: f32, to| {
            AnimNum::new(from, to)
                .time(140)
                .ease(Ease::Out)
                .function(Function::Cubic)
        });

        let target_border = if self.selected {
            theme::ACCENT_RING
        } else if hovered() {
            theme::BORDER_HOVER
        } else {
            theme::BORDER_SUBTLE
        };
        let border_state = use_reactive(&target_border);
        let border_anim = use_animation_transition(border_state, |from: Color, to| {
            AnimColor::new(from, to)
                .time(160)
                .ease(Ease::Out)
                .function(Function::Cubic)
        });

        let glow_target = if self.selected {
            0.36
        } else if hovered() {
            0.16
        } else {
            0.0
        };
        let glow_state = use_reactive(&glow_target);
        let glow_anim = use_animation_transition(glow_state, |from: f32, to| {
            AnimNum::new(from, to)
                .time(180)
                .ease(Ease::Out)
                .function(Function::Expo)
        });

        let on_select = self.on_select.clone();
        let quick_cb = self.on_quick_action.clone();
        let quick_lbl = self.quick_action_label;
        let glow_alpha = glow_anim.get().value();
        let shadow_color = if self.selected {
            theme::ACCENT.with_af32(glow_alpha)
        } else {
            Color::BLACK.with_af32(glow_alpha * 1.8)
        };

        rect()
            .width(Size::flex(1.0))
            .padding(Gaps::new(4.0, 0.0, 4.0, 0.0))
            .opacity(enter_opacity)
            .offset_y(enter_y + lift_anim.get().value())
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(134.0))
                    .corner_radius(theme::RADIUS_CARD)
                    .overflow(Overflow::Clip)
                    .background(theme::BG_CARD)
                    .shadow(Shadow::new().y(8.0).blur(22.0).color(shadow_color))
                    .scale(scale_anim.get().value())
                    .cursor(CursorIcon::Pointer)
                    .on_pointer_enter(move |_| try_set_bool(&mut hovered, true))
                    .on_pointer_leave(move |_| {
                        try_set_bool(&mut hovered, false);
                        try_set_bool(&mut pressed, false);
                    })
                    .on_pointer_down(move |_| try_set_bool(&mut pressed, true))
                    .on_press(move |_| {
                        try_set_bool(&mut pressed, false);
                        on_select.call();
                    })
                    .child(
                        ImageViewer::new((self.banner_id, self.banner_bytes))
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
                            .vertical()
                            .main_align(Alignment::SpaceBetween)
                            .padding(Gaps::new(10.0, 12.0, 12.0, 13.0))
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .horizontal()
                                    .main_align(Alignment::SpaceBetween)
                                    .cross_align(Alignment::Center)
                                    .child(match &self.top_badge {
                                        Some(badge) => rect()
                                            .padding(Gaps::new(3.0, 8.5, 3.0, 8.5))
                                            .corner_radius(6.0)
                                            .background(theme::BG_PILL_SCRIM)
                                            .border(
                                                Border::new()
                                                    .width(1.0)
                                                    .fill(Color::from_argb(26, 255, 255, 255)),
                                            )
                                            .child(
                                                label()
                                                    .text(badge.clone())
                                                    .color(theme::TEXT_PRIMARY)
                                                    .font_size(11.0)
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .max_lines(1),
                                            )
                                            .into_element(),
                                        None => rect()
                                            .width(Size::px(1.0))
                                            .height(Size::px(1.0))
                                            .into_element(),
                                    })
                                    .maybe_child(quick_lbl.zip(quick_cb).map(|(lbl, cb)| {
                                        ActionButton::new(lbl, move || cb.call())
                                            .icon(theme::PHOSPHOR_REFRESH)
                                            .variant(ButtonVariant::Primary)
                                            .compact(true)
                                    })),
                            )
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .vertical()
                                    .spacing(2.0)
                                    .child(
                                        label()
                                            .width(Size::fill())
                                            .text(self.title.clone())
                                            .color(theme::TEXT_PRIMARY)
                                            .font_size(19.0)
                                            .font_weight(FontWeight::BOLD)
                                            .letter_spacing(-0.4)
                                            .max_lines(1)
                                            .text_overflow(TextOverflow::Ellipsis),
                                    )
                                    .child(
                                        label()
                                            .width(Size::fill())
                                            .text(self.subtitle.clone())
                                            .color(if self.subtitle_warn {
                                                theme::STATUS_WARN
                                            } else {
                                                Color::from_rgb(174, 182, 196)
                                            })
                                            .font_size(11.5)
                                            .max_lines(1)
                                            .text_overflow(TextOverflow::Ellipsis),
                                    ),
                            ),
                    )
                    .child(
                        rect()
                            .position(Position::new_absolute().top(0.0).left(0.0))
                            .width(Size::percent(100.0))
                            .height(Size::percent(100.0))
                            .corner_radius(theme::RADIUS_CARD)
                            .border(
                                Border::new()
                                    .width(if self.selected { 2.0 } else { 1.0 })
                                    .fill(border_anim.get().value())
                                    .alignment(BorderAlignment::Inner),
                            ),
                    ),
            )
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}

#[derive(Clone, PartialEq)]
pub struct DisguisePresetCard {
    pub title: String,
    pub jar_preview: String,
    pub selected: bool,
    pub index: usize,
    pub on_select: NoArgCallback<()>,
    pub key: DiffKey,
}

impl KeyExt for DisguisePresetCard {
    fn key_mut(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for DisguisePresetCard {
    fn render(&self) -> impl IntoElement {
        let mut hovered = use_state(|| false);
        let mut pressed = use_state(|| false);

        let dur = 190 + (self.index.min(8) as u64) * 24;
        let entrance = use_animation(move |conf| {
            conf.on_creation(OnCreation::Run);
            (
                AnimNum::new(0.0, 1.0)
                    .time(dur)
                    .ease(Ease::Out)
                    .function(Function::Expo),
                AnimNum::new(8.0, 0.0)
                    .time(dur)
                    .ease(Ease::Out)
                    .function(Function::Expo),
            )
        });
        let (opacity, enter_y) = entrance.get().value();

        let lift_target = if hovered() && !pressed() { -2.5 } else { 0.0 };
        let lift_state = use_reactive(&lift_target);
        let lift_anim = use_animation_transition(lift_state, |from: f32, to| {
            AnimNum::new(from, to)
                .time(170)
                .ease(Ease::Out)
                .function(Function::Expo)
        });

        let target_bg = if self.selected {
            theme::ACCENT_SOFT
        } else if hovered() {
            theme::BG_HOVER
        } else {
            theme::BG_CARD
        };
        let target_border = if self.selected {
            theme::ACCENT_RING
        } else if hovered() {
            theme::BORDER_HOVER
        } else {
            theme::BORDER_SUBTLE
        };

        let bg_state = use_reactive(&target_bg);
        let border_state = use_reactive(&target_border);
        let bg_anim = use_animation_transition(bg_state, |from: Color, to| {
            AnimColor::new(from, to)
                .time(150)
                .ease(Ease::Out)
                .function(Function::Cubic)
        });
        let border_anim = use_animation_transition(border_state, |from: Color, to| {
            AnimColor::new(from, to)
                .time(150)
                .ease(Ease::Out)
                .function(Function::Cubic)
        });

        let scale_target = if pressed() { 0.975 } else { 1.0 };
        let scale_state = use_reactive(&scale_target);
        let scale_anim = use_animation_transition(scale_state, |from: f32, to| {
            AnimNum::new(from, to)
                .time(110)
                .ease(Ease::Out)
                .function(Function::Cubic)
        });

        let on_select = self.on_select.clone();

        rect()
            .width(Size::flex(1.0))
            .padding(Gaps::new(3.0, 0.0, 3.0, 0.0))
            .opacity(opacity)
            .offset_y(enter_y + lift_anim.get().value())
            .child(
                rect()
                    .width(Size::fill())
                    .vertical()
                    .spacing(4.0)
                    .padding(Gaps::new(12.0, 14.0, 12.0, 14.0))
                    .corner_radius(theme::RADIUS_CARD)
                    .background(bg_anim.get().value())
                    .border(
                        Border::new()
                            .width(if self.selected { 1.5 } else { 1.0 })
                            .fill(border_anim.get().value())
                            .alignment(BorderAlignment::Inner),
                    )
                    .scale(scale_anim.get().value())
                    .cursor(CursorIcon::Pointer)
                    .on_pointer_enter(move |_| try_set_bool(&mut hovered, true))
                    .on_pointer_leave(move |_| {
                        try_set_bool(&mut hovered, false);
                        try_set_bool(&mut pressed, false);
                    })
                    .on_pointer_down(move |_| try_set_bool(&mut pressed, true))
                    .on_press(move |_| {
                        try_set_bool(&mut pressed, false);
                        on_select.call();
                    })
                    .child(
                        label()
                            .text(self.title.clone())
                            .color(theme::TEXT_PRIMARY)
                            .font_size(13.5)
                            .font_weight(FontWeight::SEMI_BOLD)
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis),
                    )
                    .child(
                        label()
                            .text(self.jar_preview.clone())
                            .font_family("Geist Mono")
                            .color(if self.selected {
                                Color::from_rgb(164, 184, 255)
                            } else {
                                theme::TEXT_MUTED
                            })
                            .font_size(11.0)
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis),
                    ),
            )
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}

#[derive(Clone, PartialEq)]
pub struct SegmentPill {
    pub label: String,
    pub selected: bool,
    pub on_press: NoArgCallback<()>,
    pub key: DiffKey,
}

impl KeyExt for SegmentPill {
    fn key_mut(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for SegmentPill {
    fn render(&self) -> impl IntoElement {
        let mut hovered = use_state(|| false);
        let target_bg = if self.selected {
            theme::ACCENT
        } else if hovered() {
            theme::BG_HOVER
        } else {
            Color::TRANSPARENT
        };
        let bg_state = use_reactive(&target_bg);
        let bg_anim = use_animation_transition(bg_state, |from: Color, to| {
            AnimColor::new(from, to)
                .time(150)
                .ease(Ease::Out)
                .function(Function::Cubic)
        });
        let on_press = self.on_press.clone();

        rect()
            .padding(Gaps::new(5.0, 12.0, 5.0, 12.0))
            .corner_radius(6.0)
            .background(bg_anim.get().value())
            .cursor(CursorIcon::Pointer)
            .on_pointer_enter(move |_| try_set_bool(&mut hovered, true))
            .on_pointer_leave(move |_| try_set_bool(&mut hovered, false))
            .on_press(move |_| on_press.call())
            .child(
                label()
                    .text(self.label.clone())
                    .color(if self.selected {
                        Color::WHITE
                    } else if hovered() {
                        theme::TEXT_PRIMARY
                    } else {
                        theme::TEXT_SECONDARY
                    })
                    .font_size(12.0)
                    .font_weight(if self.selected {
                        FontWeight::SEMI_BOLD
                    } else {
                        FontWeight::MEDIUM
                    })
                    .max_lines(1),
            )
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}

#[derive(Clone, PartialEq)]
pub struct SkeletonCardGrid;

impl Component for SkeletonCardGrid {
    fn render(&self) -> impl IntoElement {
        let pulse = use_animation(|conf| {
            conf.on_creation(OnCreation::Run);
            conf.on_finish(OnFinish::reverse());
            AnimNum::new(0.4, 0.85)
                .time(850)
                .ease(Ease::InOut)
                .function(Function::Sine)
        });
        let alpha = pulse.get().value();

        rect()
            .width(Size::fill())
            .vertical()
            .spacing(12.0)
            .opacity(alpha)
            .children((0..2).map(|_| {
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::Flex)
                    .spacing(12.0)
                    .children((0..3).map(|_| {
                        rect()
                            .width(Size::flex(1.0))
                            .height(Size::px(134.0))
                            .corner_radius(theme::RADIUS_CARD)
                            .background(theme::BG_CARD)
                            .border(Border::new().width(1.0).fill(theme::BORDER_SUBTLE))
                    }))
            }))
    }
}

#[derive(Clone, PartialEq)]
pub struct ToggleSwitch {
    pub checked: bool,
    pub on_toggle: NoArgCallback<()>,
}

impl Component for ToggleSwitch {
    fn render(&self) -> impl IntoElement {
        let thumb_target = if self.checked { 19.0 } else { 3.0 };
        let thumb_state = use_reactive(&thumb_target);
        let thumb_anim = use_animation_transition(thumb_state, |from: f32, to| {
            AnimNum::new(from, to)
                .time(190)
                .ease(Ease::Out)
                .function(Function::Expo)
        });

        let track_target = if self.checked {
            theme::ACCENT
        } else {
            theme::BG_HOVER
        };
        let track_state = use_reactive(&track_target);
        let track_anim = use_animation_transition(track_state, |from: Color, to| {
            AnimColor::new(from, to)
                .time(170)
                .ease(Ease::Out)
                .function(Function::Cubic)
        });

        let on_toggle = self.on_toggle.clone();

        rect()
            .width(Size::px(38.0))
            .height(Size::px(21.0))
            .rounded_full()
            .horizontal()
            .cross_align(Alignment::Center)
            .background(track_anim.get().value())
            .border(
                Border::new()
                    .width(1.0)
                    .fill(if self.checked {
                        theme::ACCENT_RING
                    } else {
                        theme::BORDER_CONTROL
                    })
                    .alignment(BorderAlignment::Inner),
            )
            .offset_x(thumb_anim.get().value())
            .cursor(CursorIcon::Pointer)
            .on_press(move |_| on_toggle.call())
            .child(
                rect()
                    .width(Size::px(15.0))
                    .height(Size::px(15.0))
                    .rounded_full()
                    .background(Color::WHITE),
            )
    }
}

#[derive(Clone, PartialEq)]
pub struct SlimProgressBar {
    pub percent: f32,
}

impl Component for SlimProgressBar {
    fn render(&self) -> impl IntoElement {
        let clamped = self.percent.clamp(0.0, 100.0);
        let pct_state = use_reactive(&clamped);
        let width_anim = use_animation_transition(pct_state, |from: f32, to| {
            AnimNum::new(from, to)
                .time(200)
                .ease(Ease::Out)
                .function(Function::Expo)
        });

        rect()
            .width(Size::fill())
            .height(Size::px(5.0))
            .rounded_full()
            .overflow(Overflow::Clip)
            .background(theme::BG_HOVER)
            .child(
                rect()
                    .width(Size::percent(width_anim.get().value()))
                    .height(Size::fill())
                    .rounded_full()
                    .background(theme::ACCENT),
            )
    }
}

#[derive(Clone, PartialEq)]
pub struct DropBackdrop {
    pub visible: bool,
}

impl Component for DropBackdrop {
    fn render(&self) -> impl IntoElement {
        let target = if self.visible { 1.0 } else { 0.0 };
        let state = use_reactive(&target);
        let anim = use_animation_transition(state, |from: f32, to| {
            AnimNum::new(from, to)
                .time(160)
                .ease(Ease::Out)
                .function(Function::Expo)
        });
        let val = anim.get().value();

        if val <= 0.01 {
            return rect().width(Size::px(0.0)).height(Size::px(0.0));
        }

        rect()
            .position(Position::new_global().top(0.0).left(0.0))
            .width(Size::window_percent(100.0))
            .height(Size::window_percent(100.0))
            .layer(Layer::Overlay)
            .background(theme::BG_TOPBAR.with_af32(0.90 * val))
            .center()
            .opacity(val)
            .child(
                rect()
                    .padding(Gaps::new(28.0, 42.0, 28.0, 42.0))
                    .corner_radius(theme::RADIUS_PANEL)
                    .background(theme::BG_PANEL)
                    .border(Border::new().width(1.5).fill(theme::ACCENT_RING))
                    .shadow(Shadow::new().y(14.0).blur(32.0).color((0, 0, 0, 170)))
                    .scale(0.95 + 0.05 * val)
                    .vertical()
                    .center()
                    .spacing(10.0)
                    .child(PhosphorIcon::new(
                        theme::PHOSPHOR_FOLDER,
                        28.0,
                        theme::ACCENT_GLOW,
                    ))
                    .child(
                        label()
                            .text("Drop instance folder to add")
                            .color(theme::TEXT_PRIMARY)
                            .font_size(15.0)
                            .font_weight(FontWeight::SEMI_BOLD),
                    )
                    .child(
                        label()
                            .text("Supports OneClient, Modrinth, Prism, .minecraft, MultiMC, and CurseForge")
                            .color(theme::TEXT_SECONDARY)
                            .font_size(12.5),
                    ),
            )
    }
}
