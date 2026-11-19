use super::Highlight as H;
use super::ShellProfile;
use super::scan::{Role, prompt_sign, scan};

/// The runs of `line` that `class` covers, in order.
fn runs(line: &str, role: Option<Role>, input_at: Option<usize>, class: H) -> Vec<String> {
    runs_in(ShellProfile::Unix, line, role, input_at, class)
}

fn runs_in(
    profile: ShellProfile,
    line: &str,
    role: Option<Role>,
    at: Option<usize>,
    class: H,
) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    scan(&chars, line, role, at, profile, &mut out);
    let mut found: Vec<String> = Vec::new();
    let mut open = false;
    for (c, &k) in chars.iter().zip(&out) {
        match (k == class as u8, open) {
            (true, true) => found.last_mut().unwrap().push(*c),
            (true, false) => found.push(c.to_string()),
            _ => {}
        }
        open = k == class as u8;
    }
    found
}

fn output(line: &str, class: H) -> Vec<String> {
    runs(line, Some(Role::Output), None, class)
}

#[test]
fn words_are_known_whole_and_in_any_case() {
    assert_eq!(output("ERROR: build failed, errors found", H::Error), ["ERROR", "failed", "errors"]);
    assert_eq!(output("tests passed ok; node", H::Success), ["passed", "ok"]);
    assert_eq!(output("warning: sudo needed", H::Warn), ["warning", "sudo"]);
    assert_eq!(output("feature not supported here", H::Error), ["not supported"]);
    assert_eq!(output("error_code nodebug", H::Error), Vec::<String>::new());
    assert_eq!(output("[DEBUG] trace", H::Debug), ["DEBUG", "trace"]);
}

#[test]
fn paths_absolute_and_relative() {
    assert_eq!(output("see /usr/bin/env and ~/src", H::Path), ["/usr/bin/env", "~/src"]);
    assert_eq!(output("edit src/main.rs then a/b/c", H::Path), ["src/main.rs", "a/b/c"]);
    assert_eq!(output(r"in C:\Users\me now", H::Path), [r"C:\Users\me"]);
    assert_eq!(output("10 bytes/sec, link/ether", H::Path), Vec::<String>::new());
}

#[test]
fn addresses_dates_and_numbers() {
    assert_eq!(output("from 192.168.0.1 to fe80::1c2d:3e4f", H::Ip), ["192.168.0.1", "fe80::1c2d:3e4f"]);
    assert_eq!(output("not 1.2.3.4.5 nor 300.1.1.1", H::Ip), Vec::<String>::new());
    assert_eq!(output("ether aa:bb:cc:dd:ee:ff up", H::Mac), ["aa:bb:cc:dd:ee:ff"]);
    assert_eq!(output("at 2024-01-15 09:30:45 and 14:30", H::DateTime), ["2024-01-15 09:30:45", "14:30"]);
    assert_eq!(output("Wed Oct 25 10:15:30 UTC 2023 done", H::DateTime), ["Wed Oct 25 10:15:30 UTC 2023"]);
    assert_eq!(output("09:30:45", H::Ip), Vec::<String>::new());
    assert_eq!(output("Jan 12/25/2023", H::DateTime), ["Jan", "12/25/2023"]);
    assert_eq!(
        output("42 -7 1.5e3 0x1F 89% file_v2 math-2", H::Number),
        ["42", "-7", "1.5e3", "0x1F", "89%"]
    );
}

#[test]
fn strings_permissions_options_and_signs() {
    assert_eq!(output(r#"say "hi there" and 'x"#, H::Quoted), [r#""hi there""#, "'x"]);
    let mode = "drwxr-sr-x  2 me";
    assert_eq!(output(mode, H::PermType), ["d"]);
    assert_eq!(output(mode, H::PermRead), ["r", "r", "r"]);
    assert_eq!(output(mode, H::PermSpecial), ["s"]);
    assert_eq!(output(mode, H::PermNone), ["-", "-"]);
    assert_eq!(output("use --help or -v, not my-file", H::Flag), ["--help", "-v"]);
    assert_eq!(output("a = (b)", H::Operator), ["="]);
    assert_eq!(output("a = (b)", H::Bracket), ["(", ")"]);
}

#[test]
fn urls_cover_what_was_there() {
    assert_eq!(output("open https://x.test/a-b?c=1 now", H::Url), ["https://x.test/a-b?c=1"]);
    assert_eq!(output("open https://x.test/a now", H::Path), Vec::<String>::new());
}

#[test]
fn prompts_are_told_by_their_look() {
    let line = "me@box:~/src$ git commit -m x | less";
    assert_eq!(runs(line, None, None, H::PromptSign), ["$"]);
    assert_eq!(runs(line, None, None, H::Path), ["~/src"]);
    assert_eq!(runs(line, None, None, H::Command), ["git", "less"]);
    assert_eq!(runs(line, None, None, H::Flag), ["-m"]);
    let cmd = r"C:\Users\John Doe\ws>dir > out.txt";
    assert_eq!(runs(cmd, None, None, H::Path), [r"C:\Users\John Doe\ws"]);
    assert_eq!(runs(cmd, None, None, H::Command), ["dir"]);
    assert_eq!(runs(r"PS C:\src> ls", None, None, H::Command), ["ls"]);
    for (line, sign) in [("[me@box ~]# ", 10), ("(venv) ~/p% ", 10), ("$ echo", 0), ("PS>", 2)] {
        assert_eq!(prompt_sign(&line.chars().collect::<Vec<_>>()), Some(sign), "{line}");
    }
    for line in
        ["100% done", "#include <stdio.h>", "$HOME=/root", "a > b", r"C:\log size > 3", r"C:\a -> C:\b"]
    {
        assert_eq!(prompt_sign(&line.chars().collect::<Vec<_>>()), None, "{line}");
    }
}

#[test]
fn a_marked_prompt_ends_where_the_input_starts() {
    let line = "PS C:\\src> git status";
    let at = line.find("git");
    assert_eq!(runs(line, Some(Role::Prompt), at, H::PromptSign), [">"]);
    assert_eq!(runs(line, Some(Role::Prompt), at, H::Command), ["git"]);
    assert_eq!(runs("user ┐ ls", Some(Role::Prompt), Some(7), H::PromptSign), ["┐"], "the last non-blank");
    assert_eq!(runs("me@box:~$ error", Some(Role::Output), None, H::PromptSign), Vec::<String>::new());
}

#[test]
fn a_shells_profile_knows_its_own_prompts_and_options() {
    let sign = |p: ShellProfile, line: &str| runs_in(p, line, None, None, H::PromptSign);
    let (cmd, pwsh, dumb, unix) =
        (ShellProfile::Cmd, ShellProfile::PowerShell, ShellProfile::Dumb, ShellProfile::Unix);
    // Continuations: Command Prompt's `>`, PowerShell's `>>`; output elsewhere (a quote).
    assert_eq!(sign(cmd, "> more"), [">"]);
    assert_eq!(sign(pwsh, ">> $x"), [">"]);
    assert!(sign(unix, "> a quoted line").is_empty());
    let unc = r"PS Microsoft.PowerShell.Core\FileSystem::\server\share> ls";
    assert_eq!(sign(pwsh, unc), [">"]);
    assert_eq!(runs_in(pwsh, unc, None, None, H::Command), ["ls"]);
    // A router's prompt, only on a dumb line.
    assert_eq!(sign(dumb, "core-sw1# show run"), ["#"]);
    assert!(sign(unix, "core-sw1# show run").is_empty());
    // `/s` is an option of Command Prompt's, a path of the others'.
    let line = r"C:\work>dir /s -a";
    assert_eq!(runs_in(cmd, line, None, None, H::Flag), ["/s", "-a"]);
    assert_eq!(runs_in(unix, line, None, None, H::Flag), ["-a"]);
}
