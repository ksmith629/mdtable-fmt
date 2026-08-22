//! Parsing and rendering for GitHub-flavored markdown tables.
//!
//! Every function here is pure: given the same input it always produces the
//! same output, and none of it touches the filesystem or stdio. That's on
//! purpose - the CLI in `main.rs` is the only part of this crate allowed to
//! do I/O, so the formatting logic can be tested with plain string
//! comparisons.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    /// No colon in the separator cell - GitHub renders this left-aligned,
    /// but we track it separately so we don't invent alignment the input
    /// didn't ask for.
    None,
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// rows[0] is the header. Every row has exactly `alignments.len()` cells.
    pub rows: Vec<Vec<String>>,
    pub alignments: Vec<Alignment>,
}

/// Splits one `|`-delimited table row into trimmed cells, honoring `\|` as
/// an escaped pipe rather than a column separator.
pub fn split_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);

    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'|') {
            current.push('|');
            chars.next();
        } else if c == '|' {
            cells.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(c);
        }
    }
    cells.push(current.trim().to_string());
    cells
}

/// A separator cell is made of optional leading/trailing colons around one
/// or more dashes, e.g. `---`, `:--`, `--:`, `:-:`.
fn is_separator_cell(cell: &str) -> bool {
    let c = cell.trim();
    let core = c.strip_prefix(':').unwrap_or(c);
    let core = core.strip_suffix(':').unwrap_or(core);
    !core.is_empty() && core.chars().all(|ch| ch == '-')
}

fn parse_alignment(cell: &str) -> Alignment {
    let c = cell.trim();
    let left = c.starts_with(':');
    let right = c.ends_with(':');
    match (left, right) {
        (true, true) => Alignment::Center,
        (true, false) => Alignment::Left,
        (false, true) => Alignment::Right,
        (false, false) => Alignment::None,
    }
}

/// Parses a markdown table out of `input`. Blank lines are ignored, so the
/// table doesn't need to be the only thing in the string. Returns `None` if
/// the first two non-blank lines don't form a valid header + separator pair.
pub fn parse_table(input: &str) -> Option<Table> {
    let lines: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() < 2 {
        return None;
    }

    let header = split_row(lines[0]);
    let separator = split_row(lines[1]);
    if header.is_empty() || separator.len() != header.len() {
        return None;
    }
    if !separator.iter().all(|c| is_separator_cell(c)) {
        return None;
    }

    let alignments: Vec<Alignment> = separator.iter().map(|c| parse_alignment(c)).collect();
    let col_count = header.len();

    let mut rows = vec![header];
    for line in &lines[2..] {
        let mut cells = split_row(line);
        cells.resize(col_count, String::new());
        cells.truncate(col_count);
        rows.push(cells);
    }

    Some(Table { rows, alignments })
}

/// GitHub requires at least three dashes worth of separator, so every
/// rendered column is at least this wide regardless of content.
const MIN_COLUMN_WIDTH: usize = 3;

fn column_widths(table: &Table) -> Vec<usize> {
    let mut widths = vec![MIN_COLUMN_WIDTH; table.alignments.len()];
    for row in &table.rows {
        for (i, cell) in row.iter().enumerate() {
            let len = cell.chars().count();
            if len > widths[i] {
                widths[i] = len;
            }
        }
    }
    widths
}

fn pad_cell(text: &str, width: usize, alignment: Alignment) -> String {
    let pad = width.saturating_sub(text.chars().count());
    match alignment {
        Alignment::Right => format!("{}{text}", " ".repeat(pad)),
        Alignment::Center => {
            let left = pad / 2;
            let right = pad - left;
            format!("{}{text}{}", " ".repeat(left), " ".repeat(right))
        }
        Alignment::Left | Alignment::None => format!("{text}{}", " ".repeat(pad)),
    }
}

fn separator_cell(width: usize, alignment: Alignment) -> String {
    match alignment {
        Alignment::None => "-".repeat(width),
        Alignment::Left => format!(":{}", "-".repeat(width - 1)),
        Alignment::Right => format!("{}:", "-".repeat(width - 1)),
        Alignment::Center => format!(":{}:", "-".repeat(width - 2)),
    }
}

/// Renders a table with every column padded to its widest cell and pipes
/// lined up down the page. This is the inverse of `parse_table`.
pub fn format_table(table: &Table) -> String {
    let widths = column_widths(table);
    let mut out = String::new();

    for (row_index, row) in table.rows.iter().enumerate() {
        let cells: Vec<String> = row
            .iter()
            .zip(&widths)
            .enumerate()
            .map(|(i, (cell, &w))| pad_cell(cell, w, table.alignments[i]))
            .collect();
        out.push_str("| ");
        out.push_str(&cells.join(" | "));
        out.push_str(" |\n");

        if row_index == 0 {
            let sep_cells: Vec<String> = widths
                .iter()
                .zip(&table.alignments)
                .map(|(&w, &a)| separator_cell(w, a))
                .collect();
            out.push_str("| ");
            out.push_str(&sep_cells.join(" | "));
            out.push_str(" |\n");
        }
    }

    out
}

/// Parses then re-renders `input`, which is the whole point of this crate.
/// Returns `None` if `input` doesn't contain a parseable table.
pub fn normalize(input: &str) -> Option<String> {
    parse_table(input).map(|t| format_table(&t))
}

/// Tries to read a table starting exactly at `lines[start]`. A table is a
/// header row immediately followed by a valid separator row, followed by
/// zero or more further non-blank rows. Returns the parsed table plus the
/// number of lines it occupied, so the caller can skip past it.
fn try_parse_table_at(lines: &[&str], start: usize) -> Option<(Table, usize)> {
    if start + 1 >= lines.len() {
        return None;
    }
    if lines[start].trim().is_empty() || lines[start + 1].trim().is_empty() {
        return None;
    }

    let header = split_row(lines[start]);
    let separator = split_row(lines[start + 1]);
    if header.is_empty() || separator.len() != header.len() {
        return None;
    }
    if !separator.iter().all(|c| is_separator_cell(c)) {
        return None;
    }

    let alignments: Vec<Alignment> = separator.iter().map(|c| parse_alignment(c)).collect();
    let col_count = header.len();

    let mut rows = vec![header];
    let mut end = start + 2;
    while end < lines.len() && !lines[end].trim().is_empty() {
        let mut cells = split_row(lines[end]);
        cells.resize(col_count, String::new());
        cells.truncate(col_count);
        rows.push(cells);
        end += 1;
    }

    Some((Table { rows, alignments }, end - start))
}

/// Formats every markdown table in `input` and leaves everything else -
/// prose, blank lines, headings between tables - untouched. This is what
/// lets a single file with several tables get formatted in one pass,
/// unlike `normalize`, which only looks for one table in the whole input.
/// Returns `None` if `input` doesn't contain any table, mirroring
/// `normalize`; a `None`/`Some` split is used instead of comparing output
/// to input because a file whose table is already correctly formatted
/// would otherwise look indistinguishable from "no table found".
pub fn normalize_document(input: &str) -> Option<String> {
    let lines: Vec<&str> = input.lines().collect();
    let mut out = String::new();
    let mut found_table = false;
    let mut i = 0;
    while i < lines.len() {
        match try_parse_table_at(&lines, i) {
            Some((table, consumed)) => {
                found_table = true;
                out.push_str(&format_table(&table));
                i += consumed;
            }
            None => {
                out.push_str(lines[i]);
                out.push('\n');
                i += 1;
            }
        }
    }
    found_table.then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_row_trims_and_drops_outer_pipes() {
        assert_eq!(
            split_row("| a | b  |"),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn split_row_honors_escaped_pipe() {
        assert_eq!(
            split_row(r"| a\|b | c |"),
            vec!["a|b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn split_row_without_outer_pipes() {
        assert_eq!(
            split_row("a | b"),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn parse_table_rejects_missing_separator() {
        let input = "| a | b |\n| 1 | 2 |\n";
        assert_eq!(parse_table(input), None);
    }

    #[test]
    fn parse_table_rejects_mismatched_column_count_in_separator() {
        let input = "| a | b |\n| --- |\n| 1 | 2 |\n";
        assert_eq!(parse_table(input), None);
    }

    #[test]
    fn parse_table_reads_alignment_markers() {
        let input = "|a|b|c|d|\n|:--|--:|:-:|---|\n|1|2|3|4|\n";
        let table = parse_table(input).unwrap();
        assert_eq!(
            table.alignments,
            vec![
                Alignment::Left,
                Alignment::Right,
                Alignment::Center,
                Alignment::None,
            ]
        );
    }

    #[test]
    fn parse_table_pads_short_rows_and_truncates_long_ones() {
        let input = "| a | b |\n| --- | --- |\n| 1 |\n| 1 | 2 | 3 |\n";
        let table = parse_table(input).unwrap();
        assert_eq!(table.rows[1], vec!["1".to_string(), String::new()]);
        assert_eq!(table.rows[2], vec!["1".to_string(), "2".to_string()]);
    }

    #[test]
    fn normalize_fixes_ragged_spacing() {
        let input = "|Name|Age|\n|-|-----|\n|Al|30|\n|Bo|9|\n";
        let expected = "\
| Name | Age |
| ---- | --- |
| Al   | 30  |
| Bo   | 9   |
";
        assert_eq!(normalize(input).unwrap(), expected);
    }

    #[test]
    fn normalize_respects_right_and_center_alignment() {
        let input = "| x | y |\n| --: | :-: |\n| 1 | ab |\n";
        let expected = "\
|   x |  y  |
| --: | :-: |
|   1 | ab  |
";
        assert_eq!(normalize(input).unwrap(), expected);
    }

    #[test]
    fn normalize_returns_none_for_non_table_input() {
        assert_eq!(normalize("just some text\nmore text\n"), None);
    }

    #[test]
    fn normalize_document_formats_every_table_and_keeps_prose() {
        let input = "\
# Team

|Name|Age|
|-|-----|
|Al|30|

Some notes in between.

|x|y|
|--:|:--|
|1|two|
";
        let expected = "\
# Team

| Name | Age |
| ---- | --- |
| Al   | 30  |

Some notes in between.

|   x | y   |
| --: | :-- |
|   1 | two |
";
        assert_eq!(normalize_document(input).unwrap(), expected);
    }

    #[test]
    fn normalize_document_returns_none_for_non_table_input() {
        let input = "just some text\nmore text\n";
        assert_eq!(normalize_document(input), None);
    }

    #[test]
    fn normalize_document_treats_back_to_back_tables_with_no_blank_line_as_one() {
        // Once a header + separator is found, every following non-blank
        // line is a data row - there's no re-detection of a new header
        // partway through. GitHub's own table parsing works the same way,
        // so two tables jammed together without a blank line between them
        // read as one table with an odd-looking row in the middle.
        let input = "\
|a|b|
|-|-|
|1|2|
|c|d|
|-|-|
|3|4|
";
        let expected = "\
| a   | b   |
| --- | --- |
| 1   | 2   |
| c   | d   |
| -   | -   |
| 3   | 4   |
";
        assert_eq!(normalize_document(input).unwrap(), expected);
    }

    #[test]
    fn format_table_round_trips_through_parse() {
        let table = Table {
            rows: vec![
                vec!["h1".to_string(), "h2".to_string()],
                vec!["v".to_string(), "value".to_string()],
            ],
            alignments: vec![Alignment::None, Alignment::Left],
        };
        let rendered = format_table(&table);
        let reparsed = parse_table(&rendered).unwrap();
        assert_eq!(reparsed, table);
    }
}
