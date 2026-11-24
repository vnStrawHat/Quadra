//! Links `assets/logo.ico` as the executable's icon (specs/16, "An executable's icon").

#[path = "../../tools/icon_resource.rs"]
mod icon_resource;

fn main() {
    icon_resource::link("assets/logo.ico");
}
