//! OSC 52 (specs/18, "Input"): a program's text goes to the clipboard; a program asks for the
//! clipboard and is answered only when the settings let it read.

use crate::session::Session;

impl Session {
    pub(crate) fn clip_store(&self, text: &str) {
        if let Some(clipboard) = self.0.clipboard.borrow().as_ref() {
            clipboard.write_text(text);
        }
    }

    pub(crate) fn clip_load(&self) {
        let allowed = self.0.settings.get().is_some_and(|it| it.get_untracked().clipboard_read);
        let text = match (allowed, self.0.clipboard.borrow().as_ref()) {
            (true, Some(clipboard)) => clipboard.read_text().unwrap_or_default(),
            _ => return,
        };
        let reply = format!("\x1b]52;c;{}\x07", base64(text.as_bytes()));
        self.write(reply.as_bytes());
    }
}

fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..4 {
            let sextet = (n >> (18 - 6 * i)) & 63;
            out.push(if i <= chunk.len() { char::from(ABC[sextet as usize]) } else { '=' });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_as_rfc_4648_says() {
        for (plain, coded) in
            [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foobar", "Zm9vYmFy")]
        {
            assert_eq!(super::base64(plain.as_bytes()), coded);
        }
    }
}
