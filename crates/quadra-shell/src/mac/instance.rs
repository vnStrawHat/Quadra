//! One instance (specs/16, "One instance"): the first process of an app listens on a socket
//! in the user's folder of temporary files; a later one hands it its arguments and ends.

use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::Duration;

use quadra_async::inbox;

use crate::{Instance, Launch};

/// How long a later process waits for the first to say it has what it was handed.
const WAIT: Duration = Duration::from_secs(2);

/// Whether this process is the first of the app named `id` for the user; a later one hands
/// the first `args` and the folder it was started in, and returns once the first has them.
// ponytail: two started in the same instant may both be the first (each finds nobody
// listening); a lock on a file beside the socket when that matters.
pub fn single_instance(id: &str, args: &[String]) -> Instance {
    let path = std::env::temp_dir().join(format!("quadra-{id}.sock"));
    if let Ok(mut first) = UnixStream::connect(&path) {
        let folder = std::env::current_dir().unwrap_or_default();
        let mut text = folder.display().to_string();
        args.iter().for_each(|arg| text.extend(['\0'].into_iter().chain(arg.chars())));
        let _ = first.set_read_timeout(Some(WAIT));
        if first.write_all(text.as_bytes()).is_ok() && first.shutdown(Shutdown::Write).is_ok() {
            let _ = first.read(&mut [0]); // the first closes when it has them
        }
        return Instance::Second;
    }
    // Nobody listens there: what a process that ended left behind is taken away.
    let _ = std::fs::remove_file(&path);
    let launches = inbox();
    if let Ok(listener) = UnixListener::bind(&path) {
        let sender = launches.sender();
        std::thread::spawn(move || {
            for mut later in listener.incoming().flatten() {
                let (mut text, _) = (String::new(), later.set_read_timeout(Some(WAIT)));
                if later.read_to_string(&mut text).is_err() {
                    continue;
                }
                let mut parts = text.split('\0').map(str::to_string);
                let folder = PathBuf::from(parts.next().unwrap_or_default());
                sender.push(Launch { folder, args: parts.collect() });
            }
        });
    }
    Instance::First(launches)
}
