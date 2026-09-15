//! Keys the dashboard answers to: `q` quits, `:` or Enter drops to the prompt, any other key clears the alert markers.

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// What the player did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// Nothing within the timeout.
    None,
    /// Leave the program.
    Quit,
    /// Drop to the prompt.
    Prompt,
    /// Any other key: acknowledge the alerts.
    Key,
    /// The terminal changed size.
    Resize,
}

/// Wait up to `timeout` for a key or a resize.
pub fn poll(timeout: Duration) -> io::Result<Input> {
    if !event::poll(timeout)? {
        return Ok(Input::None);
    }
    match event::read()? {
        Event::Key(key) if key.kind != KeyEventKind::Release => Ok(classify(key)),
        Event::Resize(_, _) => Ok(Input::Resize),
        _ => Ok(Input::None),
    }
}

/// Map a key press to an input.
pub fn classify(key: KeyEvent) -> Input {
    match key.code {
        KeyCode::Char('c') | KeyCode::Char('d')
            if key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            Input::Quit
        }
        KeyCode::Char('q') | KeyCode::Char('Q') => Input::Quit,
        KeyCode::Char(':') | KeyCode::Enter => Input::Prompt,
        _ => Input::Key,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn test_classify_keys() {
        assert_eq!(
            classify(press(KeyCode::Char('q'), KeyModifiers::NONE)),
            Input::Quit
        );
        assert_eq!(
            classify(press(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Input::Quit
        );
        assert_eq!(
            classify(press(KeyCode::Char(':'), KeyModifiers::NONE)),
            Input::Prompt
        );
        assert_eq!(
            classify(press(KeyCode::Char(':'), KeyModifiers::SHIFT)),
            Input::Prompt
        );
        assert_eq!(
            classify(press(KeyCode::Enter, KeyModifiers::NONE)),
            Input::Prompt
        );
        assert_eq!(
            classify(press(KeyCode::Char(' '), KeyModifiers::NONE)),
            Input::Key
        );
        assert_eq!(
            classify(press(KeyCode::Char('c'), KeyModifiers::NONE)),
            Input::Key
        );
        assert_eq!(
            classify(press(KeyCode::Esc, KeyModifiers::NONE)),
            Input::Key
        );
    }
}
