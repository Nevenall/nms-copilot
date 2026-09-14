//! Terminal layout helpers: measuring visible width and placing rendered blocks side by side.

use unicode_width::UnicodeWidthStr;

/// Remove ANSI CSI escape sequences (`ESC [ ... final byte`) so a coloured line can be measured.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for c in chars.by_ref() {
                if ('\u{40}'..='\u{7e}').contains(&c) {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Width of a line as a terminal shows it: escape sequences are skipped and wide glyphs count double.
pub fn visible_width(line: &str) -> usize {
    strip_ansi(line).width()
}

/// The terminal's column count, or `None` when stdout is not a terminal.
pub fn terminal_width() -> Option<usize> {
    use std::io::IsTerminal;
    if !std::io::stdout().is_terminal() {
        return None;
    }
    terminal_size::terminal_size().map(|(w, _)| usize::from(w.0))
}

/// Arrange rendered blocks left to right in rows of at most `width` columns.
///
/// A block goes to the right of the previous one when it fits, otherwise it starts a new row. Blocks in a row are top-aligned and separated by `gap` spaces; rows are separated by a blank line. Empty lines at the start and end of each block are dropped. With `width` of `None` the blocks are simply concatenated, which is the stacked layout.
pub fn side_by_side(blocks: &[String], width: Option<usize>, gap: usize) -> String {
    let Some(width) = width else {
        return blocks.concat();
    };

    let trimmed: Vec<Vec<&str>> = blocks
        .iter()
        .map(|block| {
            let lines: Vec<&str> = block.lines().collect();
            let start = lines
                .iter()
                .position(|l| !l.is_empty())
                .unwrap_or(lines.len());
            let end = lines
                .iter()
                .rposition(|l| !l.is_empty())
                .map_or(start, |i| i + 1);
            lines[start..end].to_vec()
        })
        .filter(|lines| !lines.is_empty())
        .collect();
    let widths: Vec<usize> = trimmed
        .iter()
        .map(|lines| lines.iter().map(|l| visible_width(l)).max().unwrap_or(0))
        .collect();

    // Pack blocks into rows.
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut row: Vec<usize> = Vec::new();
    let mut row_width = 0;
    for (i, &w) in widths.iter().enumerate() {
        let needed = if row.is_empty() {
            w
        } else {
            row_width + gap + w
        };
        if !row.is_empty() && needed > width {
            rows.push(std::mem::take(&mut row));
            row_width = 0;
        }
        row_width = if row.is_empty() {
            w
        } else {
            row_width + gap + w
        };
        row.push(i);
    }
    if !row.is_empty() {
        rows.push(row);
    }

    let mut out = String::new();
    for (r, row) in rows.iter().enumerate() {
        if r > 0 {
            out.push('\n');
        }
        let height = row.iter().map(|&i| trimmed[i].len()).max().unwrap_or(0);
        for line_idx in 0..height {
            // Blocks that have already ended need no padding once nothing follows them on this line.
            let last = row
                .iter()
                .rposition(|&i| line_idx < trimmed[i].len())
                .unwrap_or(0);
            let mut line = String::new();
            for (pos, &i) in row.iter().enumerate().take(last + 1) {
                if pos > 0 {
                    line.push_str(&" ".repeat(gap));
                }
                let text = trimmed[i].get(line_idx).copied().unwrap_or("");
                line.push_str(text);
                if pos < last {
                    let pad = widths[i].saturating_sub(visible_width(text));
                    line.push_str(&" ".repeat(pad));
                }
            }
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_removes_colour_codes() {
        assert_eq!(strip_ansi("\u{1b}[31mred\u{1b}[0m plain"), "red plain");
        assert_eq!(strip_ansi("\u{1b}[38;2;19;47;77mx\u{1b}[m"), "x");
        assert_eq!(strip_ansi("no codes"), "no codes");
    }

    #[test]
    fn visible_width_counts_wide_glyphs_and_ignores_codes() {
        assert_eq!(visible_width("abc"), 3);
        assert_eq!(visible_width("\u{1b}[31mabc\u{1b}[0m"), 3);
        assert_eq!(visible_width("\u{1F611}"), 2);
    }

    #[test]
    fn none_width_concatenates() {
        let blocks = vec!["\na\n\n".to_string(), "\nb\n\n".to_string()];
        assert_eq!(side_by_side(&blocks, None, 2), "\na\n\n\nb\n\n");
    }

    #[test]
    fn blocks_that_fit_share_a_row_top_aligned() {
        let blocks = vec![
            "\nAAAA\nAA\n\n".to_string(),
            "\nBB\nBBBB\nB\n\n".to_string(),
        ];
        let out = side_by_side(&blocks, Some(20), 2);
        assert_eq!(out, "AAAA  BB\nAA    BBBB\n      B\n");
    }

    #[test]
    fn blocks_that_do_not_fit_start_a_new_row() {
        let blocks = vec![
            "AAAA\n".to_string(),
            "BBBB\n".to_string(),
            "CC\n".to_string(),
        ];
        let out = side_by_side(&blocks, Some(9), 2);
        assert_eq!(out, "AAAA\n\nBBBB  CC\n");
    }

    #[test]
    fn short_middle_block_keeps_later_blocks_aligned() {
        let blocks = vec![
            "AA\nAA\n".to_string(),
            "B\n".to_string(),
            "CC\nCC\n".to_string(),
        ];
        let out = side_by_side(&blocks, Some(20), 1);
        assert_eq!(out, "AA B CC\nAA   CC\n");
    }

    #[test]
    fn padding_uses_visible_width() {
        let blocks = vec![
            "\u{1b}[31mAA\u{1b}[0m\nAAAA\n".to_string(),
            "B\n".to_string(),
        ];
        let out = side_by_side(&blocks, Some(20), 1);
        assert_eq!(out, "\u{1b}[31mAA\u{1b}[0m   B\nAAAA\n");
    }

    #[test]
    fn empty_blocks_are_skipped() {
        let blocks = vec![String::new(), "\n\n".to_string(), "X\n".to_string()];
        assert_eq!(side_by_side(&blocks, Some(10), 2), "X\n");
    }
}
