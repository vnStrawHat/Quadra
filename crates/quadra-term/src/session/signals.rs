//! The signals of a session an app reads: what the program said of itself, kept on the UI
//! thread.

use oneterm_vt::Progress;
use quadra_reactive::Signal;

use super::Session;
use crate::status::ProgramState;

impl Session {
    /// The program's title (OSC 0 and 2); empty until it sets one.
    pub fn title(&self) -> Signal<String> {
        self.0.title
    }

    /// The working directory the shell says (OSC 7).
    pub fn cwd(&self) -> Signal<String> {
        self.0.cwd
    }

    /// How many bells rang; the app sets it back to 0 when they were seen.
    pub fn bells(&self) -> Signal<u32> {
        self.0.bells
    }

    /// OSC 9;4: none, or the program's progress.
    pub fn progress(&self) -> Signal<Option<Progress>> {
        self.0.progress
    }

    /// OSC 7501: the strongest state the program reported.
    pub fn status(&self) -> Signal<ProgramState> {
        self.0.status
    }

    /// OSC 7501: the id of the record that state is (`deploy`), empty when it has none.
    pub fn status_name(&self) -> Signal<String> {
        self.0.status_name
    }

    /// What the link says of itself in a few words (a remote one's latency); written by
    /// whoever made the link, empty for a local program.
    pub fn link_note(&self) -> Signal<String> {
        self.0.link_note
    }
}
