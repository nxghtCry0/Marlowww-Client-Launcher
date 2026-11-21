#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod github;
mod minecraft;
mod models;
mod settings;
mod theme;
mod ui;

use freya::prelude::*;

fn main() {
    launch(
        LaunchConfig::new()
            .with_font("Geist", Bytes::from_static(theme::GEIST_FONT))
            .with_font("Geist Mono", Bytes::from_static(theme::GEIST_MONO_FONT))
            .with_default_font("Geist")
            .with_window(
                WindowConfig::new(ui::app)
                    .with_title("Marlowww Client Launcher")
                    .with_size(1120.0, 730.0)
                    .with_min_size(940.0, 620.0)
                    .with_background(theme::BG_CANVAS)
                    .with_icon(LaunchConfig::window_icon(theme::WINDOW_ICON)),
            ),
    );
}
