use arboard::Clipboard;
use enigo::{Enigo, Keyboard, Settings, Key, Direction};

pub fn paste_text(text: &str) -> Result<(), String> {
    // Save current clipboard content (best effort)
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Clipboard init error: {}", e))?;

    let previous = clipboard.get_text().ok();

    // Set new text
    clipboard
        .set_text(text)
        .map_err(|e| format!("Clipboard set error: {}", e))?;

    // Small delay to ensure clipboard is ready
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Simulate paste shortcut
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("Enigo init error: {}", e))?;

    if cfg!(target_os = "macos") {
        enigo
            .key(Key::Meta, Direction::Press)
            .map_err(|e| format!("Key press error: {}", e))?;
        enigo
            .key(Key::Unicode('v'), Direction::Click)
            .map_err(|e| format!("Key click error: {}", e))?;
        enigo
            .key(Key::Meta, Direction::Release)
            .map_err(|e| format!("Key release error: {}", e))?;
    } else {
        enigo
            .key(Key::Control, Direction::Press)
            .map_err(|e| format!("Key press error: {}", e))?;
        enigo
            .key(Key::Unicode('v'), Direction::Click)
            .map_err(|e| format!("Key click error: {}", e))?;
        enigo
            .key(Key::Control, Direction::Release)
            .map_err(|e| format!("Key release error: {}", e))?;
    }

    // Small delay then restore clipboard (best effort)
    std::thread::sleep(std::time::Duration::from_millis(100));
    if let Some(prev) = previous {
        let _ = clipboard.set_text(&prev);
    }

    Ok(())
}
