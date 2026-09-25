# srs-fmt

A formatter for spaced repetition scheduling data.

Every card scheduler ends up exporting its review state somewhere: a
front/back pair, an interval, an ease factor, a due date. In practice the
files that come out of these exports are inconsistent. One tool uses
semicolons, another commas. Ease factors show up as `250` in one row and
`2,5` in the next. Dates arrive as `2024-03-12`, `12/03/2024`, or
`12.03.2024` depending on locale. Anything downstream that wants to merge or
diff scheduling data has to deal with all of that first.

`srs-fmt` reads rows shaped like `front, back, interval, ease, due_date` and
writes them out in one canonical, tab-separated form:

```
front	back	interval_days	ease	due_date
```

- `interval_days` is a non-negative integer.
- `ease` is the ease factor times 100 (`250` means 2.50).
- `due_date` is `YYYY-MM-DD`, or empty for a card that has never been
  scheduled.

If the first row looks like a header (its interval and ease columns aren't
numbers, e.g. `front,back,interval,ease,due_date`), it's skipped with a note
on stderr rather than being treated as a bad row. This check only ever
applies to the first row of the file.

## Strict by default

By default the formatter assumes the input is already close to canonical and
rejects anything that isn't: rows must be tab-separated, fields must not have
stray whitespace, ease and interval must be plain integers, and dates must be
valid ISO `YYYY-MM-DD`. The first bad row stops the run with a line number
and a reason, so a corrupted export doesn't get silently reshaped into
something wrong.

```
$ cat cards.tsv
What is the capital of France?	Paris	3	250	2024-03-12

$ cargo run -- cards.tsv
What is the capital of France?	Paris	3	250	2024-03-12
```

```
$ cat messy.csv
 What is the capital of France? ;Paris;3;2,5;12/03/2024

$ cargo run -- messy.csv
srs-fmt: line 1: field 'front' has leading or trailing whitespace
srs-fmt: pass --lenient to salvage what can be parsed
```

## Lenient mode

Pass `--lenient` to accept the messier shapes that real exports produce
instead of failing on them: the delimiter is guessed from `\t`, `;`, `,` or
`|`; fields are trimmed; ease factors may be written as decimals with a
comma or dot, or as a percentage (`250%`); intervals may be written as a
count plus a calendar unit instead of a day count (`3w`, `1mo`, `2y` — month
and year are calendar approximations of 30 and 365 days); and dates may use
`/` or `.` separators (assumed day-first when ambiguous), a month name in
either `12 Mar 2024` or `March 12, 2024` order, and a 2-digit year (`00`-`68`
reads as `2000`-`2068`, `69`-`99` as `1969`-`1999`). Rows that still can't be
parsed are skipped with a warning on stderr rather than aborting the whole
file.

```
$ cargo run -- --lenient messy.csv
What is the capital of France?	Paris	3	250	2024-03-12
srs-fmt: 1 row(s) parsed, 0 header row(s) skipped, 0 error(s)
```

Every run prints a one-line summary to stderr once it finishes (or hits the
row that stopped it in strict mode): how many rows were written, how many
header rows were skipped, and how many rows had errors.

## Usage

```
srs-fmt [--lenient] [-o OUTPUT] [INPUT]
```

Reads from `INPUT` or stdin, writes to `OUTPUT` or stdout.

## Status

Early skeleton. Parsing covers the five core fields, header detection,
lenient date handling for slash/dot/month-name formats with 2- or 4-digit
years, and lenient interval/ease shorthand, with unit tests covering both
the strict and lenient paths. Every run ends with a summary of rows parsed,
header rows skipped, and errors. Still no support for merging multiple
input files.
