//! Links `assets/logo.ico` as the executable's icon (specs/16, "An executable's icon").

fn main() {
    quadra_build::icon("assets/logo.ico");
}
