//! The dashboard's colours, taken from the deep-space palette the REPL's tables use, so the screen sits with the rest of the output rather than beside it.
//!
//! The hexes come from `nms_query::table`'s theme: the navy the tables sit on, the blue of their title and header bars, and the text blues of their rows. The gold is the banner's star, used here for anything wanting attention.

use ratatui::style::{Color, Modifier, Style};

/// The navy the tables sit on.
const BACKGROUND: Color = Color::Rgb(0x0A, 0x19, 0x29);
/// The tables' title bar.
const BAR: Color = Color::Rgb(0x1E, 0x3A, 0x5F);
/// The tables' header bar, and the panel borders.
const RULE: Color = Color::Rgb(0x2C, 0x5F, 0x8A);
/// The tables' title text.
const BRIGHT: Color = Color::Rgb(0xE0, 0xF0, 0xFF);
/// The tables' header text.
const LABEL: Color = Color::Rgb(0xA0, 0xC8, 0xE0);
/// The tables' row text.
const TEXT: Color = Color::Rgb(0xB0, 0xD0, 0xE8);
/// The tables' footer text, for anything secondary.
const MUTED: Color = Color::Rgb(0x4A, 0x9B, 0xC7);
/// The banner's star.
const ATTENTION: Color = Color::Rgb(0xDC, 0xC8, 0x64);

/// Every style the dashboard draws with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    /// Painted over the whole screen first, so the dashboard is one block of colour.
    pub background: Style,
    /// The top line, painted as a title bar.
    pub bar: Style,
    /// The program's name in that bar.
    pub title: Style,
    /// The context line in that bar.
    pub header: Style,
    /// The key reminders at the right of that bar.
    pub keys: Style,
    /// Panel borders.
    pub border: Style,
    /// Panel titles.
    pub panel_title: Style,
    /// Column headings inside a panel.
    pub column: Style,
    /// Ordinary text.
    pub text: Style,
    /// Placeholders and anything secondary.
    pub muted: Style,
    /// An alert that arrived since the last keypress.
    pub attention: Style,
}

impl Palette {
    /// The deep-space palette, for a terminal that shows colour.
    pub fn dark() -> Self {
        let on_navy = Style::default().bg(BACKGROUND);
        Self {
            background: on_navy,
            bar: Style::default().bg(BAR),
            title: Style::default()
                .bg(BAR)
                .fg(BRIGHT)
                .add_modifier(Modifier::BOLD),
            header: Style::default().bg(BAR).fg(LABEL),
            keys: Style::default().bg(BAR).fg(MUTED),
            border: on_navy.fg(RULE),
            panel_title: on_navy.fg(LABEL).add_modifier(Modifier::BOLD),
            column: Style::default().bg(RULE).fg(LABEL),
            text: on_navy.fg(TEXT),
            muted: on_navy.fg(MUTED),
            attention: on_navy.fg(ATTENTION).add_modifier(Modifier::BOLD),
        }
    }

    /// Structure without colour, for a terminal or a test that does not want it.
    pub fn plain() -> Self {
        let bold = Style::default().add_modifier(Modifier::BOLD);
        Self {
            background: Style::default(),
            bar: Style::default(),
            title: bold,
            header: Style::default(),
            keys: Style::default(),
            border: Style::default(),
            panel_title: bold,
            column: bold,
            text: Style::default(),
            muted: Style::default(),
            attention: bold,
        }
    }

    /// The deep-space palette when `color`, plain otherwise.
    pub fn for_color(color: bool) -> Self {
        if color { Self::dark() } else { Self::plain() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_palette_paints_on_the_table_navy() {
        let palette = Palette::dark();
        assert_eq!(palette.background.bg, Some(BACKGROUND));
        assert_eq!(palette.text.bg, Some(BACKGROUND));
        assert_eq!(palette.border.fg, Some(RULE));
        assert!(palette.attention.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_plain_palette_has_no_colour() {
        let palette = Palette::plain();
        for style in [
            palette.background,
            palette.bar,
            palette.title,
            palette.header,
            palette.keys,
            palette.border,
            palette.panel_title,
            palette.column,
            palette.text,
            palette.muted,
            palette.attention,
        ] {
            assert_eq!(style.fg, None);
            assert_eq!(style.bg, None);
        }
    }

    #[test]
    fn test_for_color_picks_the_palette() {
        assert_eq!(Palette::for_color(true), Palette::dark());
        assert_eq!(Palette::for_color(false), Palette::plain());
    }
}
