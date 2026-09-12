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
comma or dot; and dates may use `/` or `.` separators (assumed day-first
when ambiguous). Rows that still can't be parsed are skipped with a warning
on stderr rather than aborting the whole file.

```
$ cargo run -- --lenient messy.csv
What is the capital of France?	Paris	3	250	2024-03-12
```

## Usage

```
srs-fmt [--lenient] [-o OUTPUT] [INPUT]
```

Reads from `INPUT` or stdin, writes to `OUTPUT` or stdout.

## Status

Early skeleton. Parsing covers the five core fields plus header detection;
see the issue tracker for planned work on richer date formats and batch
statistics.
