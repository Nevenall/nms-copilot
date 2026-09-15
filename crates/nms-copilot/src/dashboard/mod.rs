//! The dashboard: a full-screen view of the alerts, bases, fleet, and log that redraws only when something shown has changed.
//!
//! The loop wakes for three things: what the watcher reports, a timer that re-checks the alerts against the clock, and keys. Each wake rebuilds the [`view::View`] and draws it only if it differs from the last one drawn, so a save write, an alert coming due, a keypress, a resize, or a coarse clock change redraws the screen and nothing else does. The model's read lock is held only while a view is built and the write lock only while a delta is applied, so the MCP server keeps answering.
//!
//! The screen is drawn in the terminal's own buffer rather than the alternate screen, in a band that leaves the last rows free. Dropping to the prompt leaves the dashboard where it is and puts the prompt underneath it, so command output scrolls the dashboard up the way any other output would, and coming back draws a fresh one at the top with everything before it kept in the scrollback.

pub mod input;
pub mod palette;
pub mod render;
pub mod view;

use std::collections::HashSet;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::{Terminal, TerminalOptions, Viewport};
use tokio::sync::RwLock;

use nms_graph::GalaxyModel;

use crate::session::{SessionState, unix_now};
use crate::watch::{self, WatchContext};
use input::Input;
use palette::Palette;
use view::View;

/// How often the watcher is checked while waiting for keys. Checking is a non-blocking channel read; it never redraws by itself.
const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Rows kept clear below the dashboard, so the prompt has somewhere to start.
pub const PROMPT_ROWS: u16 = 2;

/// Where to go when the dashboard closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// Drop to the prompt.
    Prompt,
    /// Leave the program.
    Quit,
}

/// How the dashboard behaves.
#[derive(Debug, Clone)]
pub struct Options {
    /// How often alerts are re-checked against the clock.
    pub tick: Duration,
    /// Ring the terminal bell when a new alert appears.
    pub bell: bool,
    /// Draw in the deep-space palette rather than plain text.
    pub color: bool,
}

/// Why one drawn screen ended.
enum Screen {
    /// The player left the dashboard.
    Leave(Next),
    /// The terminal changed size; draw a fresh screen at the new size.
    Resized,
}

/// The rows the dashboard occupies in a terminal of this height, leaving [`PROMPT_ROWS`] free below.
pub fn viewport_height(terminal_height: u16) -> u16 {
    let height = terminal_height.max(1);
    terminal_height.saturating_sub(PROMPT_ROWS).clamp(1, height)
}

/// Show the dashboard until the player leaves it.
///
/// Each screen is drawn in a band at the top of the terminal with the rows below it left clear; a resize starts a new one.
pub fn run(
    model: &RwLock<GalaxyModel>,
    session: &mut SessionState,
    watch: &WatchContext<'_>,
    options: &Options,
) -> io::Result<Next> {
    // Alerts already on screen when the dashboard opens are not news; only later ones get a marker. The set outlives a resize.
    let mut seen: HashSet<String> = view::alert_keys(session);
    loop {
        match run_screen(model, session, watch, options, &mut seen)? {
            Screen::Leave(next) => return Ok(next),
            Screen::Resized => {}
        }
    }
}

/// Draw one screen and run its loop, restoring the terminal on the way out even if the loop fails.
fn run_screen(
    model: &RwLock<GalaxyModel>,
    session: &mut SessionState,
    watch: &WatchContext<'_>,
    options: &Options,
    seen: &mut HashSet<String>,
) -> io::Result<Screen> {
    let mut stdout = io::stdout();
    scroll_into_history(&mut stdout)?;
    let height = viewport_height(crossterm::terminal::size()?.1);

    crossterm::terminal::enable_raw_mode()?;
    execute!(stdout, Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(height),
        },
    )?;

    let result = screen_loop(&mut terminal, model, session, watch, options, seen);

    crossterm::terminal::disable_raw_mode()?;
    // Leave the cursor on the first clear row under the dashboard, which is where the prompt draws itself.
    execute!(terminal.backend_mut(), Show, MoveTo(0, height))?;
    terminal.backend_mut().flush()?;
    result
}

fn screen_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    model: &RwLock<GalaxyModel>,
    session: &mut SessionState,
    watch: &WatchContext<'_>,
    options: &Options,
    seen: &mut HashSet<String>,
) -> io::Result<Screen> {
    let palette = Palette::for_color(options.color);
    let mut last_drawn: Option<View> = None;
    let mut last_tick = Instant::now();
    let mut wake = true;

    loop {
        let events = watch::drain(watch.receiver);
        if wake || !events.is_empty() || last_tick.elapsed() >= options.tick {
            let now = unix_now();
            let report = watch::sync(
                model,
                session,
                events,
                watch.cache_path,
                watch.save_version,
                now,
            );
            last_tick = Instant::now();
            wake = false;
            if options.bell && report.new_alerts > 0 {
                ring_bell(terminal)?;
            }
            let view = {
                let guard = model.blocking_read();
                view::build(&guard, session, seen, now)
            };
            if last_drawn.as_ref() != Some(&view) {
                terminal.draw(|frame| render::draw(frame, &view, &palette))?;
                last_drawn = Some(view);
            }
        }

        let until_tick = options.tick.saturating_sub(last_tick.elapsed());
        match input::poll(until_tick.min(POLL_INTERVAL))? {
            Input::None => {}
            Input::Quit => return Ok(Screen::Leave(Next::Quit)),
            Input::Prompt => return Ok(Screen::Leave(Next::Prompt)),
            Input::Key => {
                *seen = view::alert_keys(session);
                wake = true;
            }
            Input::Resize => return Ok(Screen::Resized),
        }
    }
}

/// Push what is on the screen up into the scrollback and put the cursor at the top, so the dashboard draws on a blank screen without erasing anything the player may want to scroll back to.
fn scroll_into_history(out: &mut impl Write) -> io::Result<()> {
    let height = crossterm::terminal::size()?.1;
    let used = crossterm::cursor::position()
        .map(|(_, y)| y.saturating_add(1))
        .unwrap_or(height)
        .min(height);
    execute!(out, MoveTo(0, height.saturating_sub(1)))?;
    out.write_all("\n".repeat(used as usize).as_bytes())?;
    execute!(out, MoveTo(0, 0))?;
    out.flush()
}

/// The terminal bell; Windows Terminal turns it into a sound, a flash, or nothing according to its own settings.
fn ring_bell(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let backend = terminal.backend_mut();
    backend.write_all(b"\x07")?;
    backend.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_height_leaves_room_for_the_prompt() {
        assert_eq!(viewport_height(24), 22);
        assert_eq!(viewport_height(50), 48);
        assert_eq!(viewport_height(3), 1);
        assert_eq!(viewport_height(2), 1, "never nothing");
        assert_eq!(viewport_height(1), 1, "never taller than the terminal");
        assert_eq!(viewport_height(0), 1);
    }
}
