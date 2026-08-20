use std::fmt;

#[derive(Debug, Clone)]
pub struct Entry {
    pub date: String,
    pub start: String,
    pub end: String,
    pub project: String,
    pub notes: String,
}

#[derive(Debug)]
pub struct ParseError {
    line: usize,
    column: usize,
    message: String,
    line_text: String,
}

impl ParseError {
    pub(crate) fn new(line: usize, column: usize, message: String, line_text: &str) -> Self {
        ParseError {
            line,
            column,
            message,
            line_text: line_text.to_string(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "line {}, column {}: {}", self.line, self.column, self.message)?;
        writeln!(f, "  {}", self.line_text)?;
        write!(f, "  {}^", " ".repeat(self.column.saturating_sub(1)))
    }
}

/// Parses the "punch" log format: one entry per line, `DATE START-END PROJECT [NOTES]`.
/// Blank lines and lines starting with `#` are comments.
pub fn parse(input: &str) -> Result<Vec<Entry>, ParseError> {
    let mut entries = Vec::new();

    for (idx, raw_line) in input.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = raw_line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let tokens = tokenize(raw_line);
        if tokens.len() < 3 {
            let col = match tokens.last() {
                Some((start, text)) => start + text.len() + 1,
                None => 1,
            };
            return Err(ParseError::new(
                line_no,
                col,
                "expected DATE START-END PROJECT [NOTES]".to_string(),
                raw_line,
            ));
        }

        let (date_col, date_tok) = tokens[0];
        if let Err(msg) = validate_date(date_tok) {
            return Err(ParseError::new(line_no, date_col + 1, msg, raw_line));
        }

        let (time_col, time_tok) = tokens[1];
        let (start, end) = match validate_time_range(time_tok) {
            Ok(pair) => pair,
            Err((offset, msg)) => {
                return Err(ParseError::new(line_no, time_col + offset + 1, msg, raw_line));
            }
        };

        let (project_col, project_tok) = tokens[2];
        let _ = project_col; // tokenize guarantees a non-empty token here
        let notes_start = tokens[2].0 + tokens[2].1.len();
        let notes = raw_line[notes_start..].trim().to_string();

        entries.push(Entry {
            date: date_tok.to_string(),
            start: start.to_string(),
            end: end.to_string(),
            project: project_tok.to_string(),
            notes,
        });
    }

    Ok(entries)
}

/// Renders entries back into punch format, e.g. for a csv -> punch conversion.
pub fn write(entries: &[Entry]) -> String {
    let mut out = String::new();
    for e in entries {
        out.push_str(&e.date);
        out.push(' ');
        out.push_str(&e.start);
        out.push('-');
        out.push_str(&e.end);
        out.push(' ');
        out.push_str(&e.project);
        if !e.notes.is_empty() {
            out.push(' ');
            out.push_str(&e.notes);
        }
        out.push('\n');
    }
    out
}

/// Splits a line on whitespace while keeping each token's byte offset, so
/// callers can turn a token-relative failure into an accurate column number.
fn tokenize(line: &str) -> Vec<(usize, &str)> {
    let mut tokens = Vec::new();
    let mut chars = line.char_indices().peekable();

    while let Some(&(start, c)) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        let mut end = start + c.len_utf8();
        chars.next();
        while let Some(&(i, c2)) = chars.peek() {
            if c2.is_whitespace() {
                break;
            }
            end = i + c2.len_utf8();
            chars.next();
        }
        tokens.push((start, &line[start..end]));
    }

    tokens
}

fn validate_time_range(tok: &str) -> Result<(&str, &str), (usize, String)> {
    let dash = tok
        .find('-')
        .ok_or_else(|| (0, format!("invalid time range \"{}\" (expected HH:MM-HH:MM)", tok)))?;
    let start = &tok[..dash];
    let end = &tok[dash + 1..];

    validate_hhmm(start).map_err(|msg| (0, msg))?;
    validate_hhmm(end).map_err(|msg| (dash + 1, msg))?;

    // HH:MM is fixed-width and zero-padded, so a plain string compare doubles
    // as a chronological one.
    if end <= start {
        return Err((
            dash + 1,
            format!(
                "end time {} must be after start time {} (overnight shifts aren't supported yet)",
                end, start
            ),
        ));
    }

    Ok((start, end))
}

pub(crate) fn validate_hhmm(s: &str) -> Result<(), String> {
    let bytes = s.as_bytes();
    if s.len() != 5 || bytes[2] != b':' {
        return Err(format!("invalid time \"{}\" (expected HH:MM)", s));
    }
    let hour: u32 = s[0..2]
        .parse()
        .map_err(|_| format!("invalid time \"{}\" (expected HH:MM)", s))?;
    let minute: u32 = s[3..5]
        .parse()
        .map_err(|_| format!("invalid time \"{}\" (expected HH:MM)", s))?;
    if hour > 23 {
        return Err(format!("invalid hour {} in time \"{}\" (must be 00-23)", hour, s));
    }
    if minute > 59 {
        return Err(format!("invalid minute {} in time \"{}\" (must be 00-59)", minute, s));
    }
    Ok(())
}

pub(crate) fn validate_date(s: &str) -> Result<(), String> {
    let bytes = s.as_bytes();
    if s.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(format!("invalid date \"{}\" (expected YYYY-MM-DD)", s));
    }
    let bad = || format!("invalid date \"{}\" (expected YYYY-MM-DD)", s);
    let year: u32 = s[0..4].parse().map_err(|_| bad())?;
    let month: u32 = s[5..7].parse().map_err(|_| bad())?;
    let day: u32 = s[8..10].parse().map_err(|_| bad())?;

    if month < 1 || month > 12 {
        return Err(format!("invalid month {} in date \"{}\"", month, s));
    }
    let max_day = days_in_month(year, month);
    if day < 1 || day > max_day {
        return Err(format!(
            "invalid day {} in date \"{}\" ({} {} has {} days)",
            day,
            s,
            month_name(month),
            year,
            max_day
        ));
    }
    Ok(())
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn month_name(month: u32) -> &'static str {
    const NAMES: [&str; 12] = [
        "January", "February", "March", "April", "May", "June", "July", "August", "September",
        "October", "November", "December",
    ];
    NAMES[(month - 1) as usize]
}
