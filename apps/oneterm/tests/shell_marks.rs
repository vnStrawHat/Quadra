//! Shell integration against the shells of this machine (specs/19, "M4 in detail"), each behind
//! a real pseudo console: Command Prompt and Windows PowerShell, which every Windows has, and
//! Git's bash where it is installed. Each says its directory and marks its prompt; PowerShell
//! and bash mark a command's start and its end with the status; all write UTF-8. Skipped where
//! no console can be made.

#![cfg(windows)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use oneterm::Shell;
use quadra_term::{Pty, PtyOptions};

/// Starts a shell as 1Term does with shell integration, runs `command` once its first prompt
/// is marked, and gives everything it wrote once all of `wanted` is in it.
fn run(
    name: &'static str,
    program: PathBuf,
    args: &[&str],
    command: &str,
    wanted: &[&str],
) -> Option<String> {
    let shell = Shell { name, program, args: args.iter().map(|it| it.to_string()).collect() };
    let mut env = Vec::new();
    let program = oneterm::integrate::start(Some(shell), &mut env);
    let out = Arc::new(Mutex::new(Vec::new()));
    let seen = out.clone();
    let options = PtyOptions { program, env, rows: 30, cols: 200, ..PtyOptions::default() };
    let spawned = Pty::spawn(
        options,
        move |bytes| {
            seen.lock().unwrap().extend_from_slice(bytes);
            Vec::new()
        },
        |_| {},
    );
    let Ok(pty) = spawned else {
        eprintln!("no pseudo console here: skipped");
        return None;
    };
    let text = || String::from_utf8_lossy(&out.lock().unwrap()).into_owned();
    let wait = |what: &[&str]| {
        let end = Instant::now() + Duration::from_secs(60);
        while !what.iter().all(|it| text().contains(it)) {
            assert!(Instant::now() < end, "{name} never said {what:?}: {:?}", text());
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    wait(&["\x1b]133;A", "\x1b]133;B"]);
    pty.write(format!("{command}\r").as_bytes());
    wait(wanted);
    pty.close();
    Some(text())
}

fn under(var: &str, path: &str) -> PathBuf {
    // With the OS's separator alone, as the shells 1Term finds are named.
    PathBuf::from(std::env::var_os(var).unwrap_or_default()).join(path).components().collect()
}

#[test]
fn command_prompt_says_its_directory_marks_its_prompt_and_writes_utf8() {
    let cmd = under("SystemRoot", "System32/cmd.exe");
    let Some(said) = run("Command Prompt", cmd, &[], "chcp", &["65001"]) else { return };
    // The directory with the prompt, and no line of the code page set at the start.
    let here = std::env::current_dir().unwrap().display().to_string();
    assert!(said.contains(&format!("\x1b]9;9;{here}")), "{said:?}");
    assert_eq!(said.matches("65001").count(), 1, "only what `chcp` answered: {said:?}");
}

#[test]
fn powershell_marks_a_command_s_start_and_its_end_with_the_status() {
    let shell = under("SystemRoot", "System32/WindowsPowerShell/v1.0/powershell.exe");
    let command = "echo marks-$([Console]::OutputEncoding.CodePage)";
    let wanted = ["marks-65001", "\x1b]133;C", "\x1b]133;D;0"];
    let Some(said) = run("Windows PowerShell", shell, &["-NoLogo"], command, &wanted) else { return };
    let here = std::env::current_dir().unwrap().display().to_string();
    assert!(said.contains(&format!("\x1b]9;9;{here}")), "{said:?}");
}

#[test]
fn git_s_bash_marks_its_commands_and_says_its_directory() {
    let bash = under("ProgramFiles", "Git/bin/bash.exe");
    if !bash.is_file() {
        eprintln!("no Git bash here: skipped");
        return;
    }
    let wanted = ["marks-ok", "\x1b]133;C", "\x1b]133;D;0", "\x1b]7;file://"];
    run("Git Bash", bash, &["--login", "-i"], "echo marks-ok", &wanted);
}
