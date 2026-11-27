mod app;
mod commands;
mod editor;
mod markdown;
mod notebook;
mod settings;
mod ui;
mod workspace;

use dioxus::desktop::{Config, WindowBuilder, tao::dpi::LogicalSize};
use dioxus::prelude::*;

use crate::commands::native;
use crate::settings::theme;

fn main() {
    let appearance = theme::Appearance::load();
    let window = WindowBuilder::new()
        .with_title("xuy")
        .with_min_inner_size(LogicalSize::new(440, 320))
        .with_theme(appearance.native());

    LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_window(window)
                .with_menu(native::build(appearance)),
        )
        .launch(app::App);
}
