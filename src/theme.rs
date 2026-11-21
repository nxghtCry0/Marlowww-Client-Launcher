use freya::prelude::*;

pub const GEIST_FONT: &[u8] = include_bytes!("../assets/fonts/Geist.ttf");
pub const GEIST_MONO_FONT: &[u8] = include_bytes!("../assets/fonts/GeistMono.ttf");
pub const WINDOW_ICON: &[u8] = include_bytes!("../assets/icon.png");
pub const OFFICIAL_LOGO: &[u8] = include_bytes!("../assets/logo.png");

pub const BANNER_WILDERNESS: &[u8] = include_bytes!("../assets/banners/wilderness.jpg");
pub const BANNER_SKY_ARENA: &[u8] = include_bytes!("../assets/banners/sky_arena.jpg");
pub const BANNER_CHERRY_GROVE: &[u8] = include_bytes!("../assets/banners/cherry_grove.jpg");
pub const BANNER_TRIAL_VAULT: &[u8] = include_bytes!("../assets/banners/trial_vault.jpg");

pub fn banner_for_version(mc_version: &str) -> (&'static str, &'static [u8]) {
    if mc_version.starts_with("26.3") {
        ("banner-wilderness-26-3", BANNER_WILDERNESS)
    } else if mc_version.starts_with("26.2") {
        ("banner-sky-arena-26-2", BANNER_SKY_ARENA)
    } else if mc_version.starts_with("26.1") {
        ("banner-cherry-grove-26-1", BANNER_CHERRY_GROVE)
    } else {
        ("banner-trial-vault-1-21", BANNER_TRIAL_VAULT)
    }
}

pub const RADIUS_PANEL: f32 = 14.0;
pub const RADIUS_CARD: f32 = 12.0;
pub const RADIUS_CONTROL: f32 = 8.0;

pub const BG_TOPBAR: Color = Color::from_rgb(10, 12, 15);
pub const BG_CANVAS: Color = Color::from_rgb(13, 15, 19);
pub const BG_PANEL: Color = Color::from_rgb(18, 22, 28);
pub const BG_CARD: Color = Color::from_rgb(21, 25, 32);
pub const BG_CONTROL: Color = Color::from_rgb(24, 29, 37);
pub const BG_HOVER: Color = Color::from_rgb(33, 40, 50);
pub const BG_PILL_SCRIM: Color = Color::from_argb(212, 11, 13, 17);

pub const DIVIDER: Color = Color::from_rgb(28, 33, 42);
pub const BORDER_SUBTLE: Color = Color::from_rgb(31, 36, 45);
pub const BORDER_CONTROL: Color = Color::from_rgb(43, 50, 62);
pub const BORDER_HOVER: Color = Color::from_rgb(66, 77, 96);

pub const ACCENT: Color = Color::from_rgb(43, 75, 254);
pub const ACCENT_HOVER: Color = Color::from_rgb(68, 99, 255);
pub const ACCENT_RING: Color = Color::from_rgb(66, 98, 255);
pub const ACCENT_GLOW: Color = Color::from_rgb(98, 126, 255);
pub const ACCENT_SOFT: Color = Color::from_rgb(20, 29, 58);

pub const TEXT_PRIMARY: Color = Color::from_rgb(246, 248, 252);
pub const TEXT_SECONDARY: Color = Color::from_rgb(148, 156, 170);
pub const TEXT_MUTED: Color = Color::from_rgb(104, 112, 126);
pub const TEXT_DISABLED: Color = Color::from_rgb(68, 75, 88);

pub const STATUS_OK: Color = Color::from_rgb(74, 222, 128);
pub const STATUS_WARN: Color = Color::from_rgb(250, 189, 47);
pub const STATUS_ERR: Color = Color::from_rgb(248, 113, 113);
pub const ERR_BG: Color = Color::from_rgb(34, 18, 22);
pub const ERR_BORDER: Color = Color::from_rgb(86, 34, 42);

pub fn canvas_ambient_gradient() -> LinearGradient {
    LinearGradient::new()
        .angle(180.0)
        .dither(true)
        .stop((Color::from_rgb(16, 19, 25), 0.0))
        .stop((BG_CANVAS, 42.0))
        .stop((Color::from_rgb(11, 13, 17), 100.0))
}

pub const PHOSPHOR_FOLDER: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><path d="M216,72H130.67L102.93,51.2a16.12,16.12,0,0,0-9.6-3.2H40A16,16,0,0,0,24,64V200a16,16,0,0,0,16,16H216.89A15.13,15.13,0,0,0,232,200.89V88A16,16,0,0,0,216,72Zm0,128H40V64H93.33L123.2,86.4A8,8,0,0,0,128,88h88Z"/></svg>"#;

pub const PHOSPHOR_REFRESH: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><path d="M224,48V96a8,8,0,0,1-8,8H168a8,8,0,0,1,0-16h28.69L182.06,73.37a79.56,79.56,0,0,0-56.13-23.43h-.45A79.52,79.52,0,0,0,69.59,72.71,8,8,0,0,1,58.41,61.27a96,96,0,0,1,135,.79L208,76.69V48a8,8,0,0,1,16,0ZM186.41,183.29a80,80,0,0,1-112.47-.66L59.31,168H88a8,8,0,0,0,0-16H40a8,8,0,0,0-8,8v48a8,8,0,0,0,16,0V179.31l14.63,14.63A95.43,95.43,0,0,0,130,222.06h.53a95.36,95.36,0,0,0,67.07-27.33,8,8,0,0,0-11.18-11.44Z"/></svg>"#;

pub const PHOSPHOR_EXTERNAL: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><path d="M200,64V168a8,8,0,0,1-16,0V83.31L69.66,197.66a8,8,0,0,1-11.32-11.32L172.69,72H88a8,8,0,0,1,0-16H192A8,8,0,0,1,200,64Z"/></svg>"#;

pub const PHOSPHOR_DOWNLOAD: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><path d="M224,144v64a8,8,0,0,1-8,8H40a8,8,0,0,1-8-8V144a8,8,0,0,1,16,0v56H208V144a8,8,0,0,1,16,0Zm-101.66,5.66a8,8,0,0,0,11.32,0l40-40a8,8,0,0,0-11.32-11.32L136,124.69V32a8,8,0,0,0-16,0v92.69L93.66,98.34a8,8,0,0,0-11.32,11.32Z"/></svg>"#;

pub const PHOSPHOR_CHECK: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><path d="M229.66,77.66l-128,128a8,8,0,0,1-11.32,0l-56-56a8,8,0,0,1,11.32-11.32L96,188.69,218.34,66.34a8,8,0,0,1,11.32,11.32Z"/></svg>"#;

pub const PHOSPHOR_SEARCH: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><path d="M229.66,218.34l-50.07-50.06a88.11,88.11,0,1,0-11.31,11.31l50.06,50.07a8,8,0,0,0,11.32-11.32ZM40,112a72,72,0,1,1,72,72A72.08,72.08,0,0,1,40,112Z"/></svg>"#;

pub const PHOSPHOR_PENCIL: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><path d="M227.31,73.37,182.63,28.68a16,16,0,0,0-22.63,0L36.69,152A15.86,15.86,0,0,0,32,163.31V208a16,16,0,0,0,16,16H92.69A15.86,15.86,0,0,0,104,219.31L227.31,96a16,16,0,0,0,0-22.63ZM92.69,208H48V163.31l88-88L180.69,120ZM192,108.68,147.31,64l24-24L216,84.68Z"/></svg>"#;

pub const PHOSPHOR_COMPASS: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm0,192a88,88,0,1,1,88-88A88.1,88.1,0,0,1,128,216ZM172.42,72.84l-64,32a8.05,8.05,0,0,0-3.58,3.58l-32,64A8,8,0,0,0,80,184a8.1,8.1,0,0,0,3.58-.84l64-32a8.05,8.05,0,0,0,3.58-3.58l32-64a8,8,0,0,0-10.74-10.74ZM138,138,97.89,158.11,118,118l40.15-20.07Z"/></svg>"#;

pub const PHOSPHOR_ARROW_RIGHT: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><path d="M221.66,133.66l-72,72a8,8,0,0,1-11.32-11.32L196.69,136H40a8,8,0,0,1,0-16H196.69L138.34,61.66a8,8,0,0,1,11.32-11.32l72,72A8,8,0,0,1,221.66,133.66Z"/></svg>"#;

pub const PHOSPHOR_ARROW_LEFT: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><path d="M224,128a8,8,0,0,1-8,8H59.31l58.35,58.34a8,8,0,0,1-11.32,11.32l-72-72a8,8,0,0,1,0-11.32l72-72a8,8,0,0,1,11.32,11.32L59.31,120H216A8,8,0,0,1,224,128Z"/></svg>"#;

pub const PHOSPHOR_CLOSE: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><path d="M205.66,194.34a8,8,0,0,1-11.32,11.32L128,139.31,61.66,205.66a8,8,0,0,1-11.32-11.32L116.69,128,50.34,61.66A8,8,0,0,1,61.66,50.34L128,116.69l66.34-66.35a8,8,0,0,1,11.32,11.32L139.31,128Z"/></svg>"#;

pub const PHOSPHOR_WARNING: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm0,192a88,88,0,1,1,88-88A88.1,88.1,0,0,1,128,216Zm-8-80V80a8,8,0,0,1,16,0v56a8,8,0,0,1-16,0Zm20,36a12,12,0,1,1-12-12A12,12,0,0,1,140,172Z"/></svg>"#;

pub const PHOSPHOR_PLUS: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" fill="currentColor"><path d="M224,128a8,8,0,0,1-8,8H136v80a8,8,0,0,1-16,0V136H40a8,8,0,0,1,0-16h80V40a8,8,0,0,1,16,0v80h80A8,8,0,0,1,224,128Z"/></svg>"#;
