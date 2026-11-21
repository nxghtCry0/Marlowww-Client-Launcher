use freya::prelude::*;

fn main() {
    launch(
        LaunchConfig::new()
            .with_window(WindowConfig::new(app).with_title("Marlowww Client Launcher")),
    );
}

fn app() -> impl IntoElement {
    rect().width(Size::fill()).height(Size::fill())
}
