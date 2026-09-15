//! Ratatui layout for the dashboard. Nothing here reads the model; it lays out the strings in a [`View`] in the colours of a [`Palette`].
//!
//! Each panel is a bordered section holding one of the REPL's tables: the section's frame and name, then a header bar of column names over rows on the deep-space navy.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

use super::palette::Palette;
use super::view::{BaseRow, FleetPanel, FleetRow, View};

/// Below this many columns the bases and fleet panels stack instead of sitting side by side. Both carry the same columns as the REPL's tables, which need the width.
pub const STACK_BELOW: u16 = 150;

/// Draw the whole screen.
pub fn draw(frame: &mut Frame, view: &View, palette: &Palette) {
    let area = frame.area();
    // One block of colour under everything, so the sections sit on the same navy as the tables.
    frame.render_widget(Block::default().style(palette.background), area);

    let rows = area.height.saturating_sub(1);
    if area.width >= STACK_BELOW {
        let middle = needs(view).bases.max(needs(view).fleet);
        let heights = share(
            rows,
            &[needs(view).alerts, middle, needs(view).log],
            &[MIN_ALERTS, MIN_TABLE, MIN_LOG],
        );
        let [header, alerts, middle, log] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(heights[0]),
            Constraint::Length(heights[1]),
            Constraint::Length(heights[2]),
        ])
        .areas(area);
        draw_header(frame, header, view, palette);
        draw_alerts(frame, alerts, view, palette);
        let [bases, fleet] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(middle);
        draw_bases(frame, bases, &view.bases, palette);
        draw_fleet(frame, fleet, view.fleet.as_ref(), palette);
        draw_log(frame, log, &view.log, palette);
    } else {
        let need = needs(view);
        let heights = share(
            rows,
            &[need.alerts, need.bases, need.fleet, need.log],
            &[MIN_ALERTS, MIN_TABLE, MIN_TABLE, MIN_LOG],
        );
        let [header, alerts, bases, fleet, log] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(heights[0]),
            Constraint::Length(heights[1]),
            Constraint::Length(heights[2]),
            Constraint::Length(heights[3]),
        ])
        .areas(area);
        draw_header(frame, header, view, palette);
        draw_alerts(frame, alerts, view, palette);
        draw_bases(frame, bases, &view.bases, palette);
        draw_fleet(frame, fleet, view.fleet.as_ref(), palette);
        draw_log(frame, log, &view.log, palette);
    }
}

/// Two borders and one line of content: the least a section can show and still say anything.
const MIN_ALERTS: u16 = 3;
/// Two borders, column headings, and one row.
const MIN_TABLE: u16 = 4;
/// Two borders and one line.
const MIN_LOG: u16 = 3;

/// The rows each section would take if the screen were tall enough.
struct Needs {
    alerts: u16,
    bases: u16,
    fleet: u16,
    log: u16,
}

fn needs(view: &View) -> Needs {
    Needs {
        // Two borders and the alerts, but never more than six of them; the rest are a keypress away at the prompt.
        alerts: (view.alerts.len().clamp(1, 6) as u16) + 2,
        // Two borders, column headings, one row per base.
        bases: view.bases.len().max(1) as u16 + 3,
        // Two borders, column headings, one row per expedition, a blank, the Navigator and home lines.
        fleet: match &view.fleet {
            Some(fleet) => fleet.rows.len().max(1) as u16 + 6,
            None => MIN_TABLE,
        },
        // Two borders and the kept log lines.
        log: view.log.len().max(1) as u16 + 2,
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

fn draw_alerts(frame: &mut Frame, area: Rect, view: &View, palette: &Palette) {
    let body = panel(frame, area, "Alerts", palette);
    let lines: Vec<Line> = if view.alerts.is_empty() {
        vec![Line::from(Span::styled("Nothing waiting", palette.muted))]
    } else {
        view.alerts
            .iter()
            .map(|alert| {
                if alert.fresh {
                    Line::from(vec![
                        Span::styled("\u{25CF} ", palette.attention),
                        Span::styled(alert.text.clone(), palette.attention),
                    ])
                } else {
                    Line::from(Span::styled(format!("  {}", alert.text), palette.text))
                }
            })
            .collect()
    };
    frame.render_widget(Paragraph::new(lines), body);
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
    let rows = bases.iter().map(|b| {
        Row::new(vec![
            b.name.clone(),
            b.kind.clone(),
            b.crops.clone(),
            b.next.clone(),
            b.extraction.clone(),
            b.power.clone(),
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
    let lines = [
        Line::from(Span::styled(fleet.offers.clone(), palette.muted)),
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
            r.number.clone(),
            r.kind.clone(),
            r.length.clone(),
            r.frigates.clone(),
            r.events.clone(),
            r.elapsed.clone(),
            r.status.clone(),
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
    use super::super::view::AlertLine;
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn sample() -> View {
        View {
            title: "NMS Copilot".into(),
            header: "Euclid \u{00B7} at Base Ferox \u{00B7} save 21:14 (10m ago) \u{00B7} watching slot 1".into(),
            keys: super::super::view::KEYS.into(),
            alerts: vec![
                AlertLine { fresh: true, text: "Fleet: expedition 1 (Trade) is waiting for your decision since 20:58".into() },
                AlertLine { fresh: false, text: "Base Ferox: 14 Gamma Weed ready".into() },
            ],
            bases: vec![
                BaseRow { name: "Base Ferox".into(), kind: "home".into(), crops: "14 / 40".into(), next: "now".into(), extraction: "3,148 / 4,000  1 of 2 FULL".into(), power: "1 battery full \u{00B7} 6 electromagnetic".into() },
                BaseRow { name: "Freighter".into(), kind: "freighter".into(), crops: "-".into(), next: "-".into(), extraction: "-".into(), power: "-".into() },
            ],
            fleet: Some(FleetPanel {
                rows: vec![FleetRow { number: "1".into(), kind: "Trade".into(), length: "Very long".into(), frigates: "5".into(), events: "16/18".into(), elapsed: "17h 30m".into(), status: "waiting since 20:58".into() }],
                offers: "Offers: 3 of 5 left \u{00B7} new in 2h 45m (00:00 UTC)".into(),
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

    /// A screen the size of a real save: eight bases, six alerts, a fleet, a full log.
    fn busy() -> View {
        let mut v = sample();
        v.alerts = (0..6)
            .map(|n| AlertLine {
                fresh: n < 2,
                text: format!("Base {n}: depots full on extraction network B (4,000 units)"),
            })
            .collect();
        v.bases = (0..8)
            .map(|n| BaseRow {
                name: format!("Base number {n}"),
                kind: "home".into(),
                crops: "16 / 142".into(),
                next: "2h 10m".into(),
                extraction: "8,898 / 9,750  2 of 3 FULL".into(),
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
        // The screen a real save fills: more alerts, bases, and log lines than a 24-row terminal can hold at once.
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
            assert!(!missing("depots full"), "no alerts at {w}x{h}:\n{text}");
        }
    }

    #[test]
    fn test_the_navigator_line_gives_way_to_the_expeditions() {
        // With room, the fleet section carries both.
        let roomy = screen(&busy(), 100, 22);
        assert!(roomy.contains("1  Trade"), "{roomy}");
        assert!(roomy.contains("Offers: 3 of 5 left"), "{roomy}");
        // With none, the expedition keeps its row and the Navigator line is the one that goes.
        let tight = screen(&busy(), 80, 20);
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
        for title in [" ALERTS ", " BASES ", " FLEET ", " LOG "] {
            assert!(text.contains(title), "missing {title} in\n{text}");
        }
        assert!(text.contains('\u{250C}'), "each section is framed:\n{text}");
        assert!(text.contains("\u{25CF} Fleet: expedition 1"), "{text}");
        assert!(text.contains("  Base Ferox: 14 Gamma Weed ready"), "{text}");
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
            alerts: Vec::new(),
            bases: Vec::new(),
            fleet: None,
            log: Vec::new(),
            ..sample()
        };
        let text = screen(&view, 120, 24);
        assert!(text.contains("Nothing waiting"), "{text}");
        assert!(text.contains("No bases"), "{text}");
        assert!(text.contains("No freighter"), "{text}");
        assert!(text.contains("Nothing yet"), "{text}");
    }

    #[test]
    fn test_tiny_screen_does_not_panic() {
        let text = screen(&sample(), 20, 5);
        assert!(!text.is_empty());
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
