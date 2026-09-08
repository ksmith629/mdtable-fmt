# mdtable-fmt

Markdown tables rot fast. You hand-edit one cell, the pipes stop lining up,
and the next diff is noise because every column shifted by a few spaces.
`mdtable-fmt` reads a table and rewrites it with consistent padding and
aligned pipes, without changing the content or the alignment you asked for.

Input:

```
|Name|Role|Years|
|-|-----|--:|
|Ada|Engineer|12|
|Grace|Rear Admiral|44|
```

Output:

```
| Name  | Role         | Years |
| ----- | ------------ | ----: |
| Ada   | Engineer     |    12 |
| Grace | Rear Admiral |    44 |
```

Column alignment (left/right/center, or none) comes from the separator row
and is preserved - `mdtable-fmt` only touches whitespace and pipe placement.

## Usage

Build with cargo (standard library only, no dependencies):

```
cargo build --release
```

Format a file:

```
mdtable-fmt notes.md
```

Or pipe a table in on stdin:

```
cat notes.md | mdtable-fmt
```

Edit a file in place instead of printing to stdout:

```
mdtable-fmt --write notes.md
```

`--write` requires a file path - there's no file to edit in place when the
input comes from stdin.

The formatted table is written to stdout. A file can contain more than one
table - each one is formatted in place, and everything in between (prose,
headings, blank lines) is passed through untouched. If the input doesn't
contain any table, the tool exits with an error instead of guessing.

## As a library

The formatting logic lives in `src/lib.rs` as plain functions with no I/O,
so it's usable outside the CLI too:

```rust
let input = "|a|b|\n|-|-|\n|1|22|\n";
let formatted = mdtable_fmt::normalize(input).unwrap();
assert_eq!(formatted, "| a   | b   |\n| --- | --- |\n| 1   | 22  |\n");
```

(columns are never narrower than three dashes, so short cells like `a`
still get padded out - that's a GitHub markdown requirement, not a bug)

`parse_table` and `format_table` are exposed separately if you need to
inspect or modify a table's cells between the two steps. `normalize` looks
for a single table anywhere in its input; `normalize_document` formats
every table in the input and passes everything else through unchanged,
which is what the CLI uses.

## Current limitations

This is an early skeleton. Known gaps:

- No handling of multi-line cells or embedded block markup.
- Column width is always "widest cell in the column" - no wrapping or
  truncation option yet.

## License

MIT, see `LICENSE`.
