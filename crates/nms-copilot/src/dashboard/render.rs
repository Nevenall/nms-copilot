//! Ratatui layout for the dashboard. Nothing here reads the model; it lays out the strings in a [`View`] in the colours of a [`Palette`].
//!
//! Each section is a bordered box with its name in the frame. The player's own details are a plain list of labelled facts, two to a line; the rest hold one of the REPL's tables, a header bar of column names over rows on the deep-space navy. A cell that wants the player is drawn in the attention colour.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use super::palette::Palette;
use super::view::{BaseRow, Fact, FleetPanel, FleetRow, PlayerPanel, View};

/// Below this many columns the bases and fleet panels stack instead of sitting side by side. Both carry the same columns as the REPL's tables, which need the width.
pub const STACK_BELOW: u16 = 150;

/// Draw the whole screen.
pub fn draw(frame: &mut Frame, view: &View, palette: &Palette) {
    let area = frame.area();
    // One block of colour under everything, so the sections sit on the same navy as the tables.
    frame.render_widget(Block::default().style(palette.background), area);

    let need = needs(view);
    let rows = area.height.saturating_sub(1);
    if area.width >= STACK_BELOW {
        let heights = share(
            rows,
            &[need.player, need.bases.max(need.fleet), need.log],
            &[MIN_PLAYER, MIN_TABLE, MIN_LOG],
        );
        let [header, player, middle, log] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(heights[0]),
            Constraint::Length(heights[1]),
            Constraint::Length(heights[2]),
        ])
        .areas(area);
        draw_header(frame, header, view, palette);
        draw_player(frame, player, &view.player, palette);
        let [bases, fleet] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(middle);
        draw_bases(frame, bases, &view.bases, palette);
        draw_fleet(frame, fleet, view.fleet.as_ref(), palette);
        draw_log(frame, log, &view.log, palette);
    } else {
        let heights = share(
            rows,
            &[need.player, need.bases, need.fleet, need.log],
            &[MIN_PLAYER, MIN_TABLE, MIN_TABLE, MIN_LOG],
        );
        let [header, player, bases, fleet, log] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(heights[0]),
            Constraint::Length(heights[1]),
            Constraint::Length(heights[2]),
            Constraint::Length(heights[3]),
        ])
        .areas(area);
        draw_header(frame, header, view, palette);
        draw_player(frame, player, &view.player, palette);
        draw_bases(frame, bases, &view.bases, palette);
        draw_fleet(frame, fleet, view.fleet.as_ref(), palette);
        draw_log(frame, log, &view.log, palette);
    }
}

/// Two borders and one line of facts.
const MIN_PLAYER: u16 = 3;
/// Two borders, column headings, and one row.
const MIN_TABLE: u16 = 4;
/// Two borders and one line.
const MIN_LOG: u16 = 3;

/// The rows each section would take if the screen were tall enough.
struct Needs {
    player: u16,
    bases: u16,
    fleet: u16,
    log: u16,
}

fn needs(view: &View) -> Needs {
    Needs {
        // Two borders and the facts, two to a line.
        player: view.player.rows().max(1) as u16 + 2,
        // Two borders, column headings, one row per base.
        bases: view.bases.len().max(1) as u16 + 3,
        // Two borders, column headings, one row per expedition, a blank, the Navigator and home lines.
        fleet: match &view.fleet {
            Some(fleet) => fleet.rows.len().max(1) as u16 + 6,
            None => MIN_TABLE,
        },
        // Two borders and the kept log lines, but asking for no more than eight: the log is history and the tables above it are the live state, so on a short screen the log is the one that yields. It still takes the slack when everything fits.
        log: (view.log.len().clamp(1, 8) as u16) + 2,
    }
}

/// Split `total` rows between sections that each want `needs[i]` and cannot usefully show less than `mins[i]`.
///
/// When everything fits, the last section keeps the slack, so spare height becomes log history. When it does not, every section is cut back towards its minimum together, so no section is starved by the ones above it.
fn share(total: u16, needs: &[u16], mins: &[u16]) -> Vec<u16> {
    let wanted: u16 = needs.iter().sum();
    if wanted <= total {
        let mut out = needs.to_vec();
        if let Some(last) = out.last_mut() {
            *last += total - wanted;
        }
        return out;
    }
    let floor: u16 = mins.iter().sum();
    if total <= floor {
        return mins.to_vec();
    }
    // Hand out what is left above the minimums in proportion to how much each section still wants.
    let mut out = mins.to_vec();
    let mut spare = total - floor;
    let hungry: u32 = needs
        .iter()
        .zip(mins)
        .map(|(need, min)| u32::from(need.saturating_sub(*min)))
        .sum();
    if hungry == 0 {
        return out;
    }
    let mut shares: Vec<(usize, u32, u16)> = needs
        .iter()
        .zip(mins)
        .enumerate()
        .map(|(i, (need, min))| {
            let want = u32::from(need.saturating_sub(*min));
            let exact = want * u32::from(spare);
            (i, exact % hungry, (exact / hungry) as u16)
        })
        .collect();
    for (i, _, whole) in &shares {
        out[*i] += *whole;
        spare -= *whole;
    }
    // Largest remainder first, so the rows that do not divide evenly go where they are most wanted.
    shares.sort_by_key(|share| std::cmp::Reverse(share.1));
    for (i, _, _) in shares.iter().take(spare as usize) {
        out[*i] += 1;
    }
    out
}

/// Frame a section and name it, and return the area left inside for its table.
fn panel(frame: &mut Frame, area: Rect, title: &str, palette: &Palette) -> Rect {
    if area.height == 0 {
        return area;
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(palette.border)
        .title(format!(" {} ", title.to_uppercase()))
        .title_style(palette.panel_title)
        .style(palette.text);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}

fn draw_header(frame: &mut Frame, area: Rect, view: &View, palette: &Palette) {
    frame.render_widget(Block::default().style(palette.bar), area);
    let keys_width = view.keys.chars().count() as u16 + 1;
    let [left, right] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(keys_width)]).areas(area);
    let line = Line::from(vec![
        Span::styled(format!(" {}", view.title), palette.title),
        Span::styled(format!("   {}", view.header), palette.header),
    ]);
    frame.render_widget(Paragraph::new(line).style(palette.bar), left);
    frame.render_widget(
        Paragraph::new(Span::styled(view.keys.clone(), palette.keys))
            .style(palette.bar)
            .right_aligned(),
        right,
    );
}

fn draw_player(frame: &mut Frame, area: Rect, player: &PlayerPanel, palette: &Palette) {
    let body = panel(frame, area, "Player", palette);
    if player.facts.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("No player state", palette.muted)),
            body,
        );
        return;
    }
    let label = |fact: Option<&Fact>| match fact {
        Some((name, _)) => Cell::from(name.clone()).style(palette.muted),
        None => Cell::from(String::new()),
    };
    let value = |fact: Option<&Fact>| match fact {
        Some((_, text)) => Cell::from(text.clone()),
        None => Cell::from(String::new()),
    };
    let rows = (0..player.rows()).map(|row| {
        let (left, right) = player.line(row);
        Row::new(vec![label(left), value(left), label(right), value(right)])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Fill(1),
            Constraint::Length(12),
            Constraint::Fill(1),
        ],
    )
    .column_spacing(1)
    .style(palette.text);
    frame.render_widget(table, body);
}

fn draw_bases(frame: &mut Frame, area: Rect, bases: &[BaseRow], palette: &Palette) {
    let body = panel(frame, area, "Bases", palette);
    if bases.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("No bases", palette.muted)),
            body,
        );
        return;
    }
    // What the alerts section used to say is said here instead: a cell the player needs to act on is drawn in the attention colour.
    let rows = bases.iter().map(|b| {
        Row::new(vec![
            Cell::from(b.name.clone()),
            Cell::from(b.kind.clone()),
            mark(&b.crops, b.crops_want_you, palette),
            mark(&b.next, b.crops_want_you, palette),
            mark(&b.extraction, b.extraction_wants_you, palette),
            Cell::from(b.power.clone()),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Fill(2),
            Constraint::Length(9),
            Constraint::Length(9),
            Constraint::Length(7),
            Constraint::Length(26),
            Constraint::Fill(3),
        ],
    )
    .header(
        Row::new(vec!["Base", "Type", "Crops", "Next", "Extraction", "Power"])
            .style(palette.column),
    )
    .column_spacing(1)
    .style(palette.text);
    frame.render_widget(table, body);
}

/// A cell in the attention colour when it wants the player, in ordinary text otherwise.
fn mark<'a>(text: &str, wants_you: bool, palette: &Palette) -> Cell<'a> {
    let cell = Cell::from(text.to_string());
    if wants_you {
        cell.style(palette.attention)
    } else {
        cell
    }
}

fn draw_fleet(frame: &mut Frame, area: Rect, fleet: Option<&FleetPanel>, palette: &Palette) {
    let body = panel(frame, area, "Fleet", palette);
    let Some(fleet) = fleet else {
        frame.render_widget(
            Paragraph::new(Span::styled("No freighter", palette.muted)),
            body,
        );
        return;
    };
    if body.height == 0 {
        return;
    }
    // The expeditions come first: the Navigator and home lines give up their rows, and then the blank between them, before a row of the table does.
    let table_rows = if fleet.rows.is_empty() {
        1
    } else {
        fleet.rows.len() as u16 + 1
    };
    let summary = 2.min(body.height.saturating_sub(table_rows));
    let gap = u16::from(body.height > table_rows + summary);
    let [table_area, _, lines_area] = Layout::vertical([
        Constraint::Length(body.height.saturating_sub(summary + gap)),
        Constraint::Length(gap),
        Constraint::Length(summary),
    ])
    .areas(body);
    if fleet.rows.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("No expeditions running", palette.muted)),
            table_area,
        );
    } else {
        frame.render_widget(
            fleet_table(&fleet.rows, palette, table_area.height),
            table_area,
        );
    }
    let offers_style = if fleet.offers_want_you {
        palette.attention
    } else {
        palette.muted
    };
    let lines = [
        Line::from(Span::styled(fleet.offers.clone(), offers_style)),
        Line::from(Span::styled(fleet.rooms.clone(), palette.muted)),
    ];
    frame.render_widget(
        Paragraph::new(lines[..summary as usize].to_vec()),
        lines_area,
    );
}

fn fleet_table<'a>(rows: &'a [FleetRow], palette: &Palette, height: u16) -> Table<'a> {
    let rows = rows.iter().map(|r| {
        Row::new(vec![
            Cell::from(r.number.clone()),
            Cell::from(r.kind.clone()),
            Cell::from(r.length.clone()),
            Cell::from(r.frigates.clone()),
            Cell::from(r.events.clone()),
            Cell::from(r.elapsed.clone()),
            mark(&r.status, r.status_wants_you, palette),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Fill(1),
        ],
    )
    .column_spacing(1)
    .style(palette.text);
    if height > 1 {
        table.header(
            Row::new(vec![
                "#", "Type", "Length", "Frigates", "Events", "Elapsed", "Status",
            ])
            .style(palette.column),
        )
    } else {
        table
    }
}

fn draw_log(frame: &mut Frame, area: Rect, log: &[String], palette: &Palette) {
    let body = panel(frame, area, "Log", palette);
    let visible = body.height as usize;
    let lines: Vec<Line> = if log.is_empty() {
        vec![Line::from(Span::styled("Nothing yet", palette.muted))]
    } else {
        // Newest at the bottom of the panel, so the eye always finds the latest line in the same place.
        let shown: Vec<Line> = log
            .iter()
            .skip(log.len().saturating_sub(visible))
            .map(|l| Line::from(Span::styled(l.clone(), palette.text)))
            .collect();
        let padding = visible.saturating_sub(shown.len());
        std::iter::repeat_n(Line::default(), padding)
            .chain(shown)
            .collect()
    };
    frame.render_widget(Paragraph::new(lines), body);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn sample() -> View {
        View {
            title: "NMS Copilot".into(),
            header: "Euclid \u{00B7} at Base Ferox \u{00B7} save 21:14 (10m ago) \u{00B7} watching slot 1".into(),
            keys: super::super::view::KEYS.into(),
            player: PlayerPanel {
                facts: [
                    ("System", "Lauderen \u{00B7} 6 planets"),
                    ("Address", "2043FC956DEC"),
                    ("From centre", "127,412 ly"),
                    ("Warped from", "Ekitok"),
                    ("Known", "293 systems \u{00B7} 644 planets"),
                    ("Units", "1,234,567"),
                    ("Nanites", "272,127"),
                    ("Quicksilver", "2,230"),
                    ("Freighter", "in this system"),
                    ("Bases", "8"),
                ]
                .map(|(label, value)| (label.to_string(), value.to_string()))
                .to_vec(),
            },
            bases: vec![
                BaseRow { name: "Base Ferox".into(), kind: "home".into(), crops: "14 / 40".into(), crops_want_you: true, next: "now".into(), extraction: "3,148 / 4,000  1 of 2 FULL".into(), extraction_wants_you: true, power: "1 battery full \u{00B7} 6 electromagnetic".into() },
                BaseRow { name: "Freighter".into(), kind: "freighter".into(), crops: "-".into(), crops_want_you: false, next: "-".into(), extraction: "-".into(), extraction_wants_you: false, power: "-".into() },
            ],
            fleet: Some(FleetPanel {
                rows: vec![FleetRow { number: "1".into(), kind: "Trade".into(), length: "Very long".into(), frigates: "5".into(), events: "16/18".into(), elapsed: "17h 30m".into(), status: "waiting since 20:58".into(), status_wants_you: true }],
                offers: "Offers: 3 of 5 left \u{00B7} new in 2h 45m (00:00 UTC)".into(),
                offers_want_you: false,
                rooms: "Rooms: 6 of 8 free \u{00B7} 17 of 25 frigates at home".into(),
            }),
            log: vec!["21:14  Save written: slot 1 Auto".into(), "20:58  Fleet: expedition 1 is waiting".into()],
        }
    }

    fn buffer_of(
        view: &View,
        palette: &Palette,
        width: u16,
        height: u16,
    ) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(frame, view, palette)).unwrap();
        terminal.backend().buffer().clone()
    }

    fn screen(view: &View, width: u16, height: u16) -> String {
        let buffer = buffer_of(view, &Palette::plain(), width, height);
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// A screen the size of a real save: eight bases, a fleet, a full log.
    fn busy() -> View {
        let mut v = sample();
        v.bases = (0..8)
            .map(|n| BaseRow {
                name: format!("Base number {n}"),
                kind: "home".into(),
                crops: "16 / 142".into(),
                crops_want_you: false,
                next: "2h 10m".into(),
                extraction: "8,898 / 9,750  2 of 3 FULL".into(),
                extraction_wants_you: true,
                power: "1 battery full \u{00B7} 6 electromagnetic".into(),
            })
            .collect();
        v.log = (0..20)
            .map(|n| format!("21:{n:02}  Save written: slot 1 Auto"))
            .collect();
        v
    }

    #[test]
    fn test_every_section_keeps_its_data_on_a_small_screen() {
        // The screen a real save fills: more bases and log lines than a 24-row terminal can hold at once.
        for (w, h) in [(100u16, 22u16), (120, 28), (80, 20)] {
            let text = screen(&busy(), w, h);
            let missing = |needle: &str| !text.contains(needle);
            // A cell from a base row: the name column is the first thing a narrow screen truncates.
            assert!(!missing("16 / 142"), "no base rows at {w}x{h}:\n{text}");
            assert!(
                !missing("1  Trade"),
                "the fleet shows its heading but no expedition at {w}x{h}:\n{text}"
            );
            assert!(!missing("Save written"), "no log lines at {w}x{h}:\n{text}");
        }
    }

    #[test]
    fn test_the_navigator_line_gives_way_to_the_expeditions() {
        // With room, the fleet section carries both.
        let roomy = screen(&busy(), 100, 22);
        assert!(roomy.contains("1  Trade"), "{roomy}");
        assert!(roomy.contains("Offers: 3 of 5 left"), "{roomy}");
        // With none, the expedition keeps its row and the Navigator line is the one that goes.
        let tight = screen(&busy(), 80, 16);
        assert!(tight.contains("1  Trade"), "{tight}");
        assert!(!tight.contains("Offers: 3 of 5 left"), "{tight}");
    }

    #[test]
    fn test_share_gives_the_slack_to_the_last_section() {
        assert_eq!(share(20, &[3, 4, 4, 3], &[3, 4, 4, 3]), vec![3, 4, 4, 9]);
        assert_eq!(share(14, &[3, 4, 4, 3], &[3, 4, 4, 3]), vec![3, 4, 4, 3]);
    }

    #[test]
    fn test_share_cuts_every_section_back_together() {
        // Wanting 26 rows in 20: each section keeps its minimum and the six spare rows go where they are most wanted.
        let out = share(20, &[5, 11, 7, 3], &[3, 4, 4, 3]);
        assert_eq!(out.iter().sum::<u16>(), 20);
        assert!(out[0] >= 3 && out[1] >= 4 && out[2] >= 4 && out[3] >= 3);
        assert!(
            out[1] > out[0],
            "the section wanting most gets most: {out:?}"
        );
    }

    #[test]
    fn test_share_never_goes_under_the_minimums() {
        assert_eq!(share(4, &[5, 11, 7, 3], &[3, 4, 4, 3]), vec![3, 4, 4, 3]);
        assert_eq!(share(0, &[5], &[3]), vec![3]);
    }

    #[test]
    fn test_panels_print_as_titled_tables() {
        let text = screen(&sample(), 120, 32);
        assert!(text.contains("NMS Copilot"), "{text}");
        assert!(text.contains("q quit  : prompt"), "{text}");
        for title in [" PLAYER ", " BASES ", " FLEET ", " LOG "] {
            assert!(text.contains(title), "missing {title} in\n{text}");
        }
        assert!(text.contains('\u{250C}'), "each section is framed:\n{text}");
    }

    #[test]
    fn test_player_section_lays_its_facts_out_two_to_a_line() {
        let text = screen(&sample(), 140, 30);
        for fact in [
            "System",
            "Lauderen",
            "Address",
            "2043FC956DEC",
            "From centre",
            "127,412 ly",
            "Units",
            "1,234,567",
            "Nanites",
            "Quicksilver",
            "Freighter",
            "in this system",
        ] {
            assert!(text.contains(fact), "missing {fact} in\n{text}");
        }
        // Ten facts make five lines, so the first of each half share the top one.
        let top = text
            .lines()
            .find(|line| line.contains("System"))
            .expect("the first line of facts");
        assert!(top.contains("Units"), "{top}");
    }

    #[test]
    fn test_base_rows_carry_the_overview_columns() {
        let text = screen(&sample(), 140, 32);
        for heading in ["Base", "Type", "Crops", "Next", "Extraction", "Power"] {
            assert!(text.contains(heading), "missing {heading} in\n{text}");
        }
        assert!(text.contains("14 / 40"), "{text}");
        assert!(text.contains("3,148 / 4,000"), "{text}");
        assert!(text.contains("1 battery full"), "{text}");
        assert!(text.contains("freighter"), "{text}");
    }

    #[test]
    fn test_fleet_rows_carry_the_overview_columns() {
        let text = screen(&sample(), 140, 32);
        for heading in ["Frigates", "Events", "Elapsed", "Status"] {
            assert!(text.contains(heading), "missing {heading} in\n{text}");
        }
        assert!(text.contains("waiting since 20:58"), "{text}");
        assert!(text.contains("Offers: 3 of 5 left"), "{text}");
        assert!(text.contains("21:14  Save written"), "{text}");
    }

    #[test]
    fn test_panels_stack_until_the_terminal_is_wide() {
        let stacked = screen(&sample(), 120, 40);
        let bases = stacked
            .lines()
            .position(|line| line.contains(" BASES "))
            .expect("bases panel");
        let fleet = stacked
            .lines()
            .position(|line| line.contains(" FLEET "))
            .expect("fleet panel");
        assert!(
            fleet > bases,
            "stacked below {STACK_BELOW} columns:\n{stacked}"
        );

        let wide = screen(&sample(), 180, 40);
        let side_by_side = wide
            .lines()
            .any(|line| line.contains(" BASES ") && line.contains(" FLEET "));
        assert!(
            side_by_side,
            "side by side above {STACK_BELOW} columns:\n{wide}"
        );
    }

    #[test]
    fn test_empty_panels_have_placeholders() {
        let view = View {
            player: PlayerPanel::default(),
            bases: Vec::new(),
            fleet: None,
            log: Vec::new(),
            ..sample()
        };
        let text = screen(&view, 120, 24);
        assert!(text.contains("No player state"), "{text}");
        assert!(text.contains("No bases"), "{text}");
        assert!(text.contains("No freighter"), "{text}");
        assert!(text.contains("Nothing yet"), "{text}");
    }

    #[test]
    fn test_tiny_screen_does_not_panic() {
        let text = screen(&sample(), 20, 5);
        assert!(!text.is_empty());
    }

    /// The style of the first character of `needle`, wherever it appears on the screen.
    fn style_at(buffer: &ratatui::buffer::Buffer, needle: &str) -> ratatui::style::Style {
        let area = buffer.area();
        for y in 0..area.height {
            let row: String = (0..area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect();
            if let Some(byte) = row.find(needle) {
                let column = row[..byte].chars().count() as u16;
                return buffer[(column, y)].style();
            }
        }
        panic!("{needle:?} is not on the screen");
    }

    #[test]
    fn test_what_wants_the_player_is_drawn_in_the_attention_colour() {
        let palette = Palette::dark();
        let buffer = buffer_of(&sample(), &palette, 140, 24);
        // Base Ferox has plants ready and a full network, so those cells and its next-harvest time carry the colour.
        assert_eq!(style_at(&buffer, "14 / 40").fg, palette.attention.fg);
        assert_eq!(style_at(&buffer, "1 of 2 FULL").fg, palette.attention.fg);
        // The trailing space keeps this off the "Known" label in the section above.
        assert_eq!(style_at(&buffer, "now ").fg, palette.attention.fg);
        // Everything else stays ordinary. The base's name is no good as a needle here: it is in the line above too.
        assert_eq!(style_at(&buffer, "freighter").fg, palette.text.fg);
        assert_eq!(style_at(&buffer, "1 battery full").fg, palette.text.fg);
        // The expedition is holding for a decision.
        assert_eq!(
            style_at(&buffer, "waiting since 20:58").fg,
            palette.attention.fg
        );
        // The Navigator has nothing new, so its line stays quiet.
        assert_eq!(style_at(&buffer, "Offers: 3 of 5").fg, palette.muted.fg);
    }

    #[test]
    fn test_dark_palette_paints_every_cell() {
        let palette = Palette::dark();
        let buffer = buffer_of(&sample(), &palette, 120, 24);
        for x in 0..120 {
            assert_eq!(
                buffer[(x, 0)].style().bg,
                palette.bar.bg,
                "the top line is a title bar"
            );
        }
        let allowed = [palette.background.bg, palette.bar.bg, palette.column.bg];
        for y in 1..24 {
            for x in 0..120 {
                let bg = buffer[(x, y)].style().bg;
                assert!(
                    allowed.contains(&bg),
                    "cell ({x}, {y}) is off the palette: {bg:?}"
                );
            }
        }
    }

    #[test]
    fn test_plain_palette_leaves_colour_alone() {
        let buffer = buffer_of(&sample(), &Palette::plain(), 120, 24);
        for y in 0..24 {
            for x in 0..120 {
                let style = buffer[(x, y)].style();
                assert_eq!(
                    style.bg,
                    Some(ratatui::style::Color::Reset),
                    "cell ({x}, {y})"
                );
                assert_eq!(
                    style.fg,
                    Some(ratatui::style::Color::Reset),
                    "cell ({x}, {y})"
                );
            }
        }
    }
}
