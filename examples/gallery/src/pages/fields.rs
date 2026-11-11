//! What text and values are typed and picked in: the inputs, the select, the date and time,
//! and a form with its rules.

use quadra::{Div, column, div, row, signal};
use quadra_kit::{
    Date, Rule, TimeOfDay, Tone, Variant, button, calendar, checkbox, combobox, date_picker, field, icons,
    input, number_input, select, textarea, time_input, validate,
};

use crate::common::{demo, hint, line, mono, page, stack};
use crate::shell::Gallery;

pub fn text_input_page() -> Div {
    let (name, search, fixed) =
        (signal(String::new()), signal(String::new()), signal("sha256:4f8a9c01".to_string()));
    let basic = stack()
        .child(input(name).placeholder("Họ và tên").width(320.0))
        .child(hint(move || format!("Bạn đã nhập: {}", name.get())));
    let states = line()
        .child(input(fixed).readonly().width(240.0))
        .child(input(signal(String::new())).placeholder("Disabled").disabled(true).width(240.0));
    let about = "One line of text with grapheme-aware editing, undo/redo, clipboard and IME composition \
                 (Telex/VNI).";
    page(
        "Text input",
        about,
        [
            demo("Basic", basic, "input(name).placeholder(\"Họ và tên\")"),
            demo(
                "With an icon",
                input(search).placeholder("Filter pods").icon(icons::SEARCH).width(320.0),
                ".icon(icons::SEARCH)",
            ),
            demo("Read-only and disabled", states, ".readonly()  .disabled(true)"),
        ],
    )
}

pub fn password_page() -> Div {
    let secret = signal(String::new());
    let field = input(secret).password().placeholder("Mật khẩu").icon(icons::LOCK).width(320.0);
    page(
        "Password input",
        "Masks what is typed and never copies it to the clipboard.",
        [demo("Basic", field, "input(secret).password().icon(icons::LOCK)")],
    )
}

pub fn number_page() -> Div {
    let (replicas, cpu) = (signal(3.0f64), signal(0.5f64));
    let whole = line().child(number_input(replicas).range(0.0, 99.0)).child(hint("replicas, 0 to 99"));
    let part = number_input(cpu).range(0.0, 16.0).step(0.25).decimals(2);
    page(
        "Number input",
        "A number with bounds and a step. Arrow keys and the stepper change it.",
        [
            demo("Integer", whole, "number_input(replicas).range(0.0, 99.0)"),
            demo(
                "Decimal",
                line().child(part).child(hint("CPU cores, step 0.25")),
                ".step(0.25).decimals(2)",
            ),
        ],
    )
}

pub fn textarea_page() -> Div {
    let notes = signal(String::new());
    let area = textarea(notes).rows(3).width(480.0);
    let area = area.placeholder("Commit message (Enter for a new line, Ctrl-Enter to commit)");
    page(
        "Textarea",
        "Several lines of text. It grows with its content.",
        [demo("Auto-growing", area, "textarea(notes).rows(3)")],
    )
}

pub fn combobox_page() -> Div {
    let towns =
        ["Hà Nội", "Hải Phòng", "Đà Nẵng", "Huế", "Cần Thơ", "Thành phố Hồ Chí Minh", "Nha Trang", "Đà Lạt"];
    let (city, picked) = (signal(String::new()), signal(""));
    let search = combobox(towns, city).placeholder("Tìm thành phố (gõ không dấu cũng được)");
    let search = search.on_pick(move |i| picked.set(towns[i]));
    let chosen = hint(move || format!("Đã chọn: {}", picked.get()));
    page(
        "Combobox",
        "A text input with a filtered list. Matching is fuzzy and ignores diacritics.",
        [demo("Search", stack().child(search).child(chosen), "combobox(options, city).on_pick(|i| ..)")],
    )
}

pub fn select_page() -> Div {
    let (space, fruit) = (signal(usize::MAX), signal(1usize));
    let spaces = select(["default", "kube-system", "monitoring", "ingress-nginx"], space);
    page(
        "Select",
        "Picks one value from a list in a popup. Escape or an outside click closes it.",
        [
            demo(
                "With a placeholder",
                line().child(spaces.placeholder("Namespace").width(220.0)),
                "select(names, chosen).placeholder(\"Namespace\")",
            ),
            demo(
                "With a value",
                line().child(select(["Táo", "Cam", "Xoài", "Sầu riêng"], fruit).width(220.0)),
                "",
            ),
        ],
    )
}

pub fn date_page() -> Div {
    let (date, day, time) =
        (signal(Date::new(2026, 11, 10)), signal(Date::new(2026, 11, 10)), signal(TimeOfDay::new(9, 30)));
    let about = "A date in a calendar popup, a calendar in the page, and a time of day typed or stepped \
                 with the arrow keys.";
    page(
        "Date and time",
        about,
        [
            demo(
                "Basic",
                line().child(date_picker(date)).child(mono(move || date.get().to_string())),
                "date_picker(since)",
            ),
            demo(
                "Calendar",
                line().child(calendar(day)).child(mono(move || day.get().to_string())),
                "calendar(day)",
            ),
            demo(
                "Time",
                line().child(time_input(time).minute_step(5)).child(mono(move || time.get().to_string())),
                "time_input(at).minute_step(5)",
            ),
        ],
    )
}

pub fn field_page(gallery: Gallery) -> Div {
    let (name, email, agree, tried) =
        (signal(String::new()), signal(String::new()), signal(false), signal(false));
    let rules = |first: &str, second: Rule| [Rule::Required(first.to_string()), second];
    let name_rules = rules("Vui lòng nhập họ tên", Rule::MinLen(3, "Tối thiểu 3 ký tự".into()));
    let mail_rules = rules("Vui lòng nhập email", Rule::Email("Email không hợp lệ".into()));
    // What is wrong shows once a submit was tried.
    let (names, mails) = (name_rules.clone(), mail_rules.clone());
    let name_wrong = move || if tried.get() { validate(&name.get(), &names) } else { String::new() };
    let mail_wrong = move || if tried.get() { validate(&email.get(), &mails) } else { String::new() };
    let (bad_name, bad_mail) = (name_wrong.clone(), mail_wrong.clone());
    let submit = move || {
        tried.set(true);
        let fine = validate(&name.get_untracked(), &name_rules).is_empty()
            && validate(&email.get_untracked(), &mail_rules).is_empty();
        if fine {
            let said = format!("Đã gửi đăng ký cho {}", name.get_untracked());
            gallery.notices.toast(Tone::Success, said);
        }
    };
    let named = input(name).placeholder("Nguyễn Văn An").width(380.0).invalid(move || !bad_name().is_empty());
    let mailed = input(email).placeholder("ban@vidu.vn").width(380.0).invalid(move || !bad_mail().is_empty());
    let send = button("Đăng ký").variant(Variant::Primary).disabled(move || !agree.get()).on_click(submit);
    let form = column()
        .gap(16.0)
        .width(380.0)
        .child(field("Họ tên", named).help("Như trên giấy tờ").error(name_wrong).required())
        .child(
            field("Email", mailed).help("Chúng tôi không chia sẻ email của bạn").error(mail_wrong).required(),
        )
        .child(checkbox(agree).label("Đồng ý điều khoản"))
        .child(row().child(send));
    let code = "field(\"Email\", control).help(..).error(move || validate(&email.get(), &rules)).required()";
    page(
        "Form field",
        "A label, a control, help text and an error. Rules validate on submit.",
        [demo("Sign-up", div().child(form), code)],
    )
}
