# punchcard-convert

I keep a daily time log in a plain text file, one line per work block,
because it's the fastest thing to append to from a terminal between
tasks. Every few weeks a client's invoicing tool wants that same data
as a CSV, and copying it over by hand is exactly the kind of
mechanical work worth automating.

This is a small command-line converter between that plain text log
format (I call it "punch" format, after a time clock) and CSV, in
either direction.

## Punch format

One entry per line:

    DATE START-END PROJECT [NOTES]

for example:

    # week of aug 17
    2026-08-18 09:00-12:15 acme-corp setup dev environment
    2026-08-18 13:00-17:30 acme-corp code review
    2026-08-19 09:15-11:00 side-project writing docs

Blank lines and lines starting with `#` are ignored. Dates are
`YYYY-MM-DD`, times are 24-hour `HH:MM`. Notes are optional and run to
the end of the line.

If the end time is earlier than the start time, the shift is assumed
to cross midnight (e.g. `22:00-02:00` is four hours), and the whole
duration is counted against the entry's date. An end time equal to
the start time is rejected, since that's ambiguous between a
zero-length shift and a full 24 hours.

## CSV format

    date,start,end,project,notes
    2026-08-18,09:00,12:15,acme-corp,setup dev environment
    2026-08-18,13:00,17:30,acme-corp,code review
    2026-08-19,09:15,11:00,side-project,writing docs

Fields containing a comma, quote, or newline are quoted the usual CSV
way (`"..."`, with `""` for an embedded quote).

## Usage

    punchcard-convert timesheet.punch timesheet.csv
    punchcard-convert timesheet.csv timesheet.punch

The format on each side is inferred from the file extension.

To see total hours instead of converting, use `--summary`:

    punchcard-convert --summary timesheet.punch

which prints total time per entry, per day, and per project:

    per entry:
      2026-08-18 09:00-12:15 acme-corp         3:15
      2026-08-18 13:00-17:30 acme-corp         4:30
      2026-08-19 09:15-11:00 side-project      1:45

    per day:
      2026-08-18   7:45
      2026-08-19   1:45

    per project:
      acme-corp        7:45
      side-project     1:45

    total: 9:30

## Error messages

Timesheets get edited by hand, so typos happen. Given a file with a
mistyped time range:

    2026-08-19 09:30-09:30 side-project writing docs

running the converter reports exactly where the problem is, not just
that there was one:

    error in timesheet.punch:
    line 1, column 18: end time 09:30 must differ from start time 09:30 (a zero-length or 24-hour shift can't be represented)
      2026-08-19 09:30-09:30 side-project writing docs
                       ^

Malformed CSV rows get the same treatment, with the column pointing at
the specific field that's wrong.

## Status

Early. Only reads and writes local files; no stdin/stdout support yet,
and overlapping time entries aren't detected.

## License

MIT, see LICENSE.
