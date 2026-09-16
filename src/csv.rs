use crate::punch::{validate_date, validate_hhmm, Entry, ParseError};

const HEADER: [&str; 5] = ["date", "start", "end", "project", "notes"];

pub fn parse(input: &str) -> Result<Vec<Entry>, ParseError> {
    let mut lines = input.lines().enumerate();

    let (_, header_line) = match lines.next() {
        Some(pair) => pair,
        None => {
            return Err(ParseError::new(
                1,
                1,
                "empty file: expected a header row (date,start,end,project,notes)".to_string(),
                "",
            ))
        }
    };

    let header_fields = split_csv_line(header_line);
    let header_ok = header_fields.len() == HEADER.len()
        && header_fields
            .iter()
            .zip(HEADER.iter())
            .all(|((_, value), expected)| value == expected);
    if !header_ok {
        return Err(ParseError::new(
            1,
            1,
            format!("expected header \"{}\"", HEADER.join(",")),
            header_line,
        ));
    }

    let mut entries = Vec::new();

    for (idx, raw_line) in lines {
        let line_no = idx + 1;
        if raw_line.trim().is_empty() {
            continue;
        }

        let fields = split_csv_line(raw_line);
        if fields.len() != HEADER.len() {
            let col = fields
                .last()
                .map(|(start, value)| start + value.len() + 1)
                .unwrap_or(1);
            return Err(ParseError::new(
                line_no,
                col,
                format!(
                    "expected {} fields (date,start,end,project,notes), found {}",
                    HEADER.len(),
                    fields.len()
                ),
                raw_line,
            ));
        }

        let (date_col, date) = &fields[0];
        if let Err(msg) = validate_date(date) {
            return Err(ParseError::new(line_no, date_col + 1, msg, raw_line));
        }
        let (start_col, start) = &fields[1];
        if let Err(msg) = validate_hhmm(start) {
            return Err(ParseError::new(line_no, start_col + 1, msg, raw_line));
        }
        let (end_col, end) = &fields[2];
        if let Err(msg) = validate_hhmm(end) {
            return Err(ParseError::new(line_no, end_col + 1, msg, raw_line));
        }
        if end == start {
            return Err(ParseError::new(
                line_no,
                end_col + 1,
                format!(
                    "end time {} must differ from start time {} (a zero-length or 24-hour shift can't be represented)",
                    end, start
                ),
                raw_line,
            ));
        }
        let (project_col, project) = &fields[3];
        if project.trim().is_empty() {
            return Err(ParseError::new(
                line_no,
                project_col + 1,
                "project must not be empty".to_string(),
                raw_line,
            ));
        }
        let (_, notes) = &fields[4];

        entries.push(Entry {
            date: date.clone(),
            start: start.clone(),
            end: end.clone(),
            project: project.clone(),
            notes: notes.clone(),
        });
    }

    Ok(entries)
}

pub fn write(entries: &[Entry]) -> String {
    let mut out = String::new();
    out.push_str("date,start,end,project,notes\n");
    for e in entries {
        write_field(&mut out, &e.date);
        out.push(',');
        write_field(&mut out, &e.start);
        out.push(',');
        write_field(&mut out, &e.end);
        out.push(',');
        write_field(&mut out, &e.project);
        out.push(',');
        write_field(&mut out, &e.notes);
        out.push('\n');
    }
    out
}

fn write_field(out: &mut String, value: &str) {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        out.push('"');
        for ch in value.chars() {
            if ch == '"' {
                out.push('"');
            }
            out.push(ch);
        }
        out.push('"');
    } else {
        out.push_str(value);
    }
}

/// Splits one CSV line into (byte offset, unescaped value) pairs, handling
/// quoted fields with embedded commas and doubled quotes.
fn split_csv_line(line: &str) -> Vec<(usize, String)> {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut fields = Vec::new();
    let mut i = 0;

    loop {
        let field_start = i;
        let mut value = String::new();

        if i < len && bytes[i] == b'"' {
            i += 1;
            while i < len {
                if bytes[i] == b'"' {
                    if i + 1 < len && bytes[i + 1] == b'"' {
                        value.push('"');
                        i += 2;
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    let ch = line[i..].chars().next().unwrap();
                    value.push(ch);
                    i += ch.len_utf8();
                }
            }
        } else {
            while i < len && bytes[i] != b',' {
                let ch = line[i..].chars().next().unwrap();
                value.push(ch);
                i += ch.len_utf8();
            }
        }

        fields.push((field_start, value));

        if i < len && bytes[i] == b',' {
            i += 1;
            continue;
        }
        break;
    }

    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER_LINE: &str = "date,start,end,project,notes\n";

    #[test]
    fn parses_valid_row_including_quoted_notes() {
        let input = format!("{}2026-08-18,09:00,12:15,acme-corp,\"hello, world\"\n", HEADER_LINE);
        let entries = parse(&input).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].notes, "hello, world");
    }

    #[test]
    fn empty_input_reports_line_one_column_one() {
        let err = parse("").unwrap_err();
        assert_eq!(err.line(), 1);
        assert_eq!(err.column(), 1);
        assert!(err.message().contains("empty file"));
    }

    #[test]
    fn wrong_header_reports_line_one_column_one() {
        let err = parse("date,start,end,project\n").unwrap_err();
        assert_eq!(err.line(), 1);
        assert_eq!(err.column(), 1);
        assert!(err.message().contains("expected header"));
    }

    #[test]
    fn missing_field_points_past_end_of_last_field() {
        let input = format!("{}2026-08-18,09:00,12:15\n", HEADER_LINE);
        let err = parse(&input).unwrap_err();
        assert_eq!(err.line(), 2);
        assert_eq!(err.column(), 23);
        assert!(err.message().contains("expected 5 fields"));
    }

    #[test]
    fn invalid_date_points_at_date_field() {
        let input = format!("{}2026-13-18,09:00,12:15,acme-corp,notes\n", HEADER_LINE);
        let err = parse(&input).unwrap_err();
        assert_eq!(err.line(), 2);
        assert_eq!(err.column(), 1);
        assert!(err.message().contains("invalid month 13"));
    }

    #[test]
    fn invalid_start_time_points_at_start_field() {
        let input = format!("{}2026-08-18,25:00,12:15,acme-corp,notes\n", HEADER_LINE);
        let err = parse(&input).unwrap_err();
        assert_eq!(err.line(), 2);
        assert_eq!(err.column(), 12);
        assert!(err.message().contains("invalid hour 25"));
    }

    #[test]
    fn zero_length_shift_points_at_end_field() {
        let input = format!("{}2026-08-18,09:00,09:00,acme-corp,notes\n", HEADER_LINE);
        let err = parse(&input).unwrap_err();
        assert_eq!(err.line(), 2);
        assert_eq!(err.column(), 18);
        assert!(err.message().contains("must differ from start time"));
    }

    #[test]
    fn empty_project_points_at_project_field() {
        let input = format!("{}2026-08-18,09:00,12:15,,notes\n", HEADER_LINE);
        let err = parse(&input).unwrap_err();
        assert_eq!(err.line(), 2);
        assert_eq!(err.column(), 24);
        assert!(err.message().contains("project must not be empty"));
    }
}
