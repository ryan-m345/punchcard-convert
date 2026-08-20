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

## Error messages

Timesheets get edited by hand, so typos happen. Given a file with a
mistyped time range:

    2026-08-19 11:00-09:30 side-project writing docs

running the converter reports exactly where the problem is, not just
that there was one:

    error in timesheet.punch:
    line 1, column 18: end time 09:30 must be after start time 11:00 (overnight shifts aren't supported yet)
      2026-08-19 11:00-09:30 side-project writing docs
                       ^

Malformed CSV rows get the same treatment, with the column pointing at
the specific field that's wrong.

## Status

Early. Dates and times are validated but not yet turned into duration
math (total hours per entry, per day, per project). See the roadmap
for what's next.

## License

MIT, see LICENSE.
