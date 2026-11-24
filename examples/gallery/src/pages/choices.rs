//! What holds a choice: the checkbox, the switch, the radio group, the segmented control and
//! the slider.

use quadra::{Div, column, div, effect, signal};
use quadra_design::{
    Size, checkbox, icon, icons, radio_group, range_slider, segmented, slider, switch, token,
};

use crate::common::{demo, line, mono, page, setting, stack};
use crate::shell::Gallery;

pub fn checkbox_page() -> Div {
    let (terms, all) = (signal(false), signal(false));
    let checks = [signal(true), signal(true), signal(false)];
    // "All" follows the three, and the three follow a click on "All".
    effect(move || all.set(checks.iter().all(|check| check.get())));
    effect(move || {
        let on = all.get();
        if on != checks.iter().all(|check| check.get_untracked()) {
            checks.iter().for_each(|check| check.set(on));
        }
    });
    let some = move || checks.iter().any(|check| check.get()) && !checks.iter().all(|check| check.get());
    let names = ["nginx", "redis", "postgres"];
    let list = column()
        .gap(token::SPACE_MD)
        .children(names.into_iter().zip(checks).map(|(name, on)| checkbox(on).label(name)));
    let several = stack()
        .child(checkbox(all).label("All services").indeterminate(some))
        .child(div().padding_left(token::SPACE_XXL).child(list));
    let states = line()
        .child(checkbox(signal(false)).label("Off"))
        .child(checkbox(signal(true)).label("On"))
        .child(checkbox(signal(false)).label("Mixed").indeterminate(true))
        .child(checkbox(signal(true)).label("Disabled").disabled(true));
    page(
        "Checkbox",
        "Turns one option on or off, or picks several from a list.",
        [
            demo(
                "Basic",
                checkbox(terms).label("Tôi đồng ý với điều khoản"),
                "checkbox(agree).label(\"..\")",
            ),
            demo("Select several", several, ".indeterminate(move || some() && !all())"),
            demo("States", states, ""),
        ],
    )
}

pub fn switch_page(gallery: Gallery) -> Div {
    let (wifi, follow) = (signal(true), signal(false));
    let settings = column()
        .max_width(520.0)
        .child(setting("Follow logs", "Scroll to new lines as they arrive", switch(follow)))
        .child(setting("Dark theme", "Applies to every window", switch(gallery.dark)))
        .child(setting("Telemetry", "Managed by your organization", switch(signal(false)).disabled(true)));
    page(
        "Switch",
        "Applies a setting immediately, without a Save button.",
        [
            demo("Basic", switch(wifi).label("Wi-Fi"), "switch(wifi).label(\"Wi-Fi\")"),
            demo("Settings", settings, ""),
        ],
    )
}

pub fn radio_page() -> Div {
    let (policy, size) = (signal(1usize), signal(0usize));
    page(
        "Radio group",
        "Exactly one choice among a few visible options. Arrow keys move the selection.",
        [
            demo(
                "Vertical",
                radio_group(["Always", "IfNotPresent", "Never"], policy),
                "radio_group([\"Always\", \"IfNotPresent\", \"Never\"], pull)",
            ),
            demo("Horizontal", radio_group(["Nhỏ", "Vừa", "Lớn"], size).horizontal(), ".horizontal()"),
            demo("Disabled", radio_group(["Read", "Write"], signal(0)).horizontal().disabled(true), ""),
            demo(
                "With hints",
                radio_group(["After 1 min", "After 5 min", "Never"], signal(0)).hints([
                    "the default",
                    "",
                    "the lock waits for you",
                ]),
                ".hints([\"the default\", \"\", ..])",
            ),
        ],
    )
}

pub fn segmented_page() -> Div {
    let (view, range, filter) = (signal(0usize), signal(1usize), signal(0usize));
    page(
        "Segmented control",
        "Switches between views of the same content.",
        [
            demo(
                "Views",
                line().child(segmented(["List", "Board", "Calendar"], view)),
                "segmented([\"List\", \"Board\", \"Calendar\"], view)",
            ),
            demo("Time range", line().child(segmented(["5m", "1h", "24h", "7d"], range)), ""),
            demo(
                "Small",
                line().child(segmented(["All", "Installed", "Not Installed"], filter).size(Size::Sm)),
                ".size(Size::Sm)",
            ),
        ],
    )
}

pub fn slider_page() -> Div {
    let (volume, temp, low, high) = (signal(40.0f32), signal(21.0f32), signal(200.0f32), signal(650.0f32));
    let mark = |body| icon(body).size(14.0).color(token::TEXT_3);
    let continuous = line()
        .child(mark(icons::MINUS))
        .child(slider(volume).width(260.0).label("Volume"))
        .child(mark(icons::PLUS))
        .child(mono(move || format!("{:.0}%", volume.get())));
    let stepped = line()
        .child(slider(temp).range(16.0, 30.0, 0.5).width(260.0).label("Temperature"))
        .child(mono(move || format!("{:.1} °C", temp.get())));
    let range = line()
        .child(range_slider(low, high).range(0.0, 1000.0, 10.0).width(260.0).label("Price"))
        .child(mono(move || format!("{:.0} – {:.0} k₫", low.get(), high.get())));
    page(
        "Slider",
        "Drag, click the track, or use the arrow keys, Page Up/Down, Home and End.",
        [
            demo("Continuous", continuous, "slider(volume)"),
            demo("Stepped", stepped, ".range(16.0, 30.0, 0.5)"),
            demo("Range", range, "range_slider(low, high)"),
            demo("Disabled", slider(signal(60.0)).width(260.0).disabled(true), ""),
        ],
    )
}
