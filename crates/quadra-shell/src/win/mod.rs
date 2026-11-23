//! The Windows shell (specs/16): the common file dialogs, the message box, opening by the
//! shell, the tray, one instance, crash reports and passwords. What shows something to the user is
//! never run by a test: those are manual checks.

mod assoc;
pub mod console;
mod crash;
mod environment;
mod instance;
mod measure;
mod process;
mod seal;
mod secret;
mod tray;
mod usage;

pub use crash::crash_reports;
pub use environment::user_environment;
pub use instance::single_instance;
pub use measure::{cycles_per_second, private_memory, process_cycles, window_shown};
pub use process::shell_process;
pub use seal::{open, random_bytes, random_key, seal};
pub use secret::{derive_key, hash_password, verify_password};
pub use usage::process_usage;

use std::path::{Path, PathBuf};

use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree,
};
use windows::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FOS_ALLOWMULTISELECT, FOS_PICKFOLDERS, FileOpenDialog, FileSaveDialog, IFileDialog, IFileOpenDialog,
    IFileSaveDialog, IShellItem, SHCreateItemFromParsingName, SIGDN_FILESYSPATH, ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    IDNO, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONQUESTION, MB_ICONWARNING, MB_OK, MB_YESNOCANCEL,
    MessageBoxW, SW_SHOWNORMAL,
};
use windows::core::{HSTRING, Interface, PCWSTR, w};

use crate::{
    Answer, Dialogs, FileDialog, Message, MessageKind, Meter, Opener, Sound, Sounds, Tray, TrayEvent,
    TrayIcon, TrayImage, Usage,
};
use tray::WinTray;

/// The shell of Windows. The dialogs make the UI thread a single-threaded COM apartment
/// when it is none yet, which they need.
pub struct WinShell;

fn apartment() {
    // SAFETY: no pointer is passed; a thread already in an apartment keeps it.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
}

fn path_of(item: &IShellItem) -> Option<PathBuf> {
    // SAFETY: the string the item gives is ours to free, once, after it is copied.
    unsafe {
        let name = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let path = name.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(name.0 as *const _));
        path
    }
}

/// Sets what every dialog of files has, then shows it over the active window. False when the
/// user gave it up, or it could not open.
fn show(dialog: &IFileDialog, of: &FileDialog) -> bool {
    let names: Vec<(HSTRING, HSTRING)> =
        of.filters.iter().map(|(name, spec)| (HSTRING::from(name), HSTRING::from(spec))).collect();
    let specs: Vec<COMDLG_FILTERSPEC> = names
        .iter()
        .map(|(name, spec)| COMDLG_FILTERSPEC {
            pszName: PCWSTR(name.as_ptr()),
            pszSpec: PCWSTR(spec.as_ptr()),
        })
        .collect();
    // SAFETY: every string outlives the calls that read it (`names`, the locals); the
    // dialog copies what it keeps.
    unsafe {
        let _ = dialog.SetTitle(&HSTRING::from(&of.title));
        if !specs.is_empty() {
            let _ = dialog.SetFileTypes(&specs);
        }
        if let Some(folder) = &of.folder
            && let Ok(item) =
                SHCreateItemFromParsingName::<_, _, IShellItem>(&HSTRING::from(folder.as_os_str()), None)
        {
            let _ = dialog.SetFolder(&item);
        }
        if let Some(start) = &of.start {
            let _ = dialog.SetFileName(&HSTRING::from(start));
        }
        let owner = GetActiveWindow();
        dialog.Show((!owner.is_invalid()).then_some(owner)).is_ok()
    }
}

fn open_dialog(of: &FileDialog, folders: bool) -> Vec<PathBuf> {
    apartment();
    // SAFETY: COM calls on an object made here, in this thread's apartment.
    unsafe {
        let Ok(dialog) = CoCreateInstance::<_, IFileOpenDialog>(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
        else {
            return Vec::new();
        };
        let mut options = dialog.GetOptions().unwrap_or_default();
        if of.multiple && !folders {
            options |= FOS_ALLOWMULTISELECT;
        }
        if folders {
            options |= FOS_PICKFOLDERS;
        }
        let _ = dialog.SetOptions(options);
        if !show(&dialog.cast().expect("an open dialog is a dialog"), of) {
            return Vec::new();
        }
        let Ok(items) = dialog.GetResults() else { return Vec::new() };
        let count = items.GetCount().unwrap_or(0);
        (0..count).filter_map(|i| items.GetItemAt(i).ok().and_then(|item| path_of(&item))).collect()
    }
}

impl Dialogs for WinShell {
    fn open_files(&self, dialog: &FileDialog) -> Vec<PathBuf> {
        open_dialog(dialog, false)
    }

    fn save_file(&self, of: &FileDialog) -> Option<PathBuf> {
        apartment();
        // SAFETY: COM calls on an object made here, in this thread's apartment.
        unsafe {
            let dialog =
                CoCreateInstance::<_, IFileSaveDialog>(&FileSaveDialog, None, CLSCTX_INPROC_SERVER).ok()?;
            if !show(&dialog.cast().ok()?, of) {
                return None;
            }
            path_of(&dialog.GetResult().ok()?)
        }
    }

    fn pick_folder(&self, dialog: &FileDialog) -> Option<PathBuf> {
        open_dialog(dialog, true).into_iter().next()
    }

    fn message(&self, message: &Message) -> Answer {
        let style = match message.kind {
            MessageKind::Info => MB_OK | MB_ICONINFORMATION,
            MessageKind::Warning => MB_OK | MB_ICONWARNING,
            MessageKind::Error => MB_OK | MB_ICONERROR,
            MessageKind::Question => MB_YESNOCANCEL | MB_ICONQUESTION,
        };
        // SAFETY: the strings live through the call.
        let answer = unsafe {
            let owner = GetActiveWindow();
            let owner = (!owner.is_invalid()).then_some(owner);
            MessageBoxW(owner, &HSTRING::from(&message.text), &HSTRING::from(&message.title), style)
        };
        match (message.kind, answer) {
            (MessageKind::Question, IDYES) => Answer::Yes,
            (MessageKind::Question, IDNO) => Answer::No,
            (MessageKind::Question, _) => Answer::Cancel,
            _ => Answer::Ok,
        }
    }
}

fn shell_open(file: &HSTRING, args: Option<&HSTRING>) -> bool {
    // SAFETY: the strings live through the call. A result above 32 is a success.
    let done = unsafe {
        let args = args.map_or(PCWSTR::null(), |a| PCWSTR(a.as_ptr()));
        ShellExecuteW(None, w!("open"), file, args, None, SW_SHOWNORMAL)
    };
    done.0 as isize > 32
}

impl Sounds for WinShell {
    fn play(&self, sound: Sound) {
        use windows::Win32::UI::WindowsAndMessaging::{MB_ICONASTERISK, MB_ICONEXCLAMATION, MB_ICONHAND};
        let kind = match sound {
            Sound::Default => MB_OK,
            Sound::Info => MB_ICONASTERISK,
            Sound::Warning => MB_ICONEXCLAMATION,
            Sound::Error => MB_ICONHAND,
            Sound::Question => MB_ICONQUESTION,
        };
        // SAFETY: no pointer is passed; the sound plays on the system's thread.
        let _ = unsafe { windows::Win32::System::Diagnostics::Debug::MessageBeep(kind) };
    }
}

impl Opener for WinShell {
    fn open(&self, target: &str) -> bool {
        apartment(); // what the shell runs for some kinds of file wants one
        shell_open(&HSTRING::from(target), None)
    }

    fn reveal(&self, path: &Path) -> bool {
        let select = HSTRING::from(format!("/select,\"{}\"", path.display()));
        shell_open(&HSTRING::from("explorer.exe"), Some(&select))
    }

    fn open_with(&self, program: &Path, args: &[String], file: &Path) -> bool {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut command = std::process::Command::new(program);
        command.args(args).arg(file).creation_flags(CREATE_NO_WINDOW);
        command.spawn().is_ok()
    }
}

impl Meter for WinShell {
    fn usage(&self) -> Option<Usage> {
        process_usage()
    }
}

impl Tray for WinShell {
    fn add(
        &self,
        image: Option<&TrayImage>,
        tooltip: &str,
        menu: &[&str],
        events: quadra_async::Sender<TrayEvent>,
    ) -> Option<Box<dyn TrayIcon>> {
        Some(Box::new(WinTray::add(image, tooltip, menu, events)?))
    }
}

impl TrayIcon for WinTray {
    fn notify(&self, title: &str, text: &str) {
        WinTray::notify(self, title, text);
    }

    fn set_tooltip(&mut self, tooltip: &str) {
        WinTray::set_tooltip(self, tooltip);
    }
}
