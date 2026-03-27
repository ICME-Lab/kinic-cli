//! Clipboard helper for host-side terminal actions.
//! Where: `tui-kit-host`, used by runtime loops that need OS clipboard access.
//! What: copies plain text into the system clipboard.
//! Why: keeps platform-specific clipboard setup out of the input loop itself.

pub fn copy_text(text: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
    clipboard
        .set_text(text.to_owned())
        .map_err(|error| error.to_string())
}
