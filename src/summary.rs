use crate::punch::{date_to_days, hhmm_to_minutes, Entry};
use std::collections::BTreeMap;

pub struct EntryDuration<'a> {
    pub entry: &'a Entry,
    pub minutes: u32,
}

pub struct Summary<'a> {
    pub entries: Vec<EntryDuration<'a>>,
    pub per_day: BTreeMap<String, u32>,
    pub per_project: BTreeMap<String, u32>,
    pub total_minutes: u32,
}

/// Computes total minutes worked per entry, per day, and per project.
/// Dates sort correctly as plain strings because they're always `YYYY-MM-DD`.
pub fn summarize(entries: &[Entry]) -> Result<Summary<'_>, String> {
    check_overlaps(entries)?;

    let mut per_day = BTreeMap::new();
    let mut per_project = BTreeMap::new();
    let mut total_minutes = 0u32;
    let mut durations = Vec::with_capacity(entries.len());

    for entry in entries {
        let minutes = duration_minutes(entry)?;
        *per_day.entry(entry.date.clone()).or_insert(0) += minutes;
        *per_project.entry(entry.project.clone()).or_insert(0) += minutes;
        total_minutes += minutes;
        durations.push(EntryDuration { entry, minutes });
    }

    Ok(Summary {
        entries: durations,
        per_day,
        per_project,
        total_minutes,
    })
}

struct Interval<'a> {
    start: i64,
    end: i64,
    entry: &'a Entry,
}

/// Rejects entries whose time ranges overlap, comparing across dates too so
/// a shift that crosses midnight is checked against the next day's entries.
/// Sorting by start and tracking the furthest end seen so far catches any
/// overlapping pair, not just ones adjacent in the sort order.
fn check_overlaps(entries: &[Entry]) -> Result<(), String> {
    let mut intervals: Vec<Interval> = entries
        .iter()
        .map(|e| {
            let day = date_to_days(&e.date);
            let start_min = hhmm_to_minutes(&e.start) as i64;
            let end_min = hhmm_to_minutes(&e.end) as i64;
            let start = day * 1440 + start_min;
            let end = if end_min <= start_min {
                (day + 1) * 1440 + end_min
            } else {
                day * 1440 + end_min
            };
            Interval { start, end, entry: e }
        })
        .collect();

    intervals.sort_by_key(|iv| iv.start);

    let mut furthest: Option<&Interval> = None;
    for iv in &intervals {
        if let Some(prev) = furthest {
            if iv.start < prev.end {
                return Err(format!(
                    "{} {}-{} {} overlaps with {} {}-{} {}",
                    prev.entry.date,
                    prev.entry.start,
                    prev.entry.end,
                    prev.entry.project,
                    iv.entry.date,
                    iv.entry.start,
                    iv.entry.end,
                    iv.entry.project
                ));
            }
        }
        if furthest.map_or(true, |prev| iv.end > prev.end) {
            furthest = Some(iv);
        }
    }
    Ok(())
}

const MINUTES_PER_DAY: u32 = 24 * 60;

fn duration_minutes(entry: &Entry) -> Result<u32, String> {
    let start = hhmm_to_minutes(&entry.start);
    let end = hhmm_to_minutes(&entry.end);
    if end == start {
        return Err(format!(
            "{} {}-{} {}: end time must differ from start time (a zero-length or 24-hour shift can't be represented)",
            entry.date, entry.start, entry.end, entry.project
        ));
    }
    if end < start {
        // crosses midnight: counted entirely against the entry's (start) date
        Ok((MINUTES_PER_DAY - start) + end)
    } else {
        Ok(end - start)
    }
}

pub fn format_hm(minutes: u32) -> String {
    format!("{}:{:02}", minutes / 60, minutes % 60)
}

pub fn render(summary: &Summary) -> String {
    let mut out = String::new();

    out.push_str("per entry:\n");
    for e in &summary.entries {
        out.push_str(&format!(
            "  {} {}-{} {:<15} {:>6}\n",
            e.entry.date,
            e.entry.start,
            e.entry.end,
            e.entry.project,
            format_hm(e.minutes)
        ));
    }

    out.push_str("\nper day:\n");
    for (day, minutes) in &summary.per_day {
        out.push_str(&format!("  {} {:>6}\n", day, format_hm(*minutes)));
    }

    out.push_str("\nper project:\n");
    for (project, minutes) in &summary.per_project {
        out.push_str(&format!("  {:<15} {:>6}\n", project, format_hm(*minutes)));
    }

    out.push_str(&format!("\ntotal: {}\n", format_hm(summary.total_minutes)));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(date: &str, start: &str, end: &str, project: &str) -> Entry {
        Entry {
            date: date.to_string(),
            start: start.to_string(),
            end: end.to_string(),
            project: project.to_string(),
            notes: String::new(),
        }
    }

    #[test]
    fn totals_per_entry_day_and_project() {
        let entries = vec![
            entry("2026-08-18", "09:00", "12:15", "acme-corp"),
            entry("2026-08-18", "13:00", "17:30", "acme-corp"),
            entry("2026-08-19", "09:15", "11:00", "side-project"),
        ];

        let summary = summarize(&entries).unwrap();

        assert_eq!(summary.entries[0].minutes, 195);
        assert_eq!(summary.entries[1].minutes, 270);
        assert_eq!(summary.entries[2].minutes, 105);

        assert_eq!(summary.per_day["2026-08-18"], 465);
        assert_eq!(summary.per_day["2026-08-19"], 105);

        assert_eq!(summary.per_project["acme-corp"], 465);
        assert_eq!(summary.per_project["side-project"], 105);

        assert_eq!(summary.total_minutes, 570);
    }

    #[test]
    fn computes_duration_across_midnight() {
        let entries = vec![entry("2026-08-18", "22:00", "02:00", "acme-corp")];
        let summary = summarize(&entries).unwrap();
        assert_eq!(summary.entries[0].minutes, 240);
        assert_eq!(summary.per_day["2026-08-18"], 240);
    }

    #[test]
    fn rejects_zero_length_shift() {
        let entries = vec![entry("2026-08-18", "11:00", "11:00", "acme-corp")];
        let err = summarize(&entries).unwrap_err();
        assert!(err.contains("must differ from start time"));
    }

    #[test]
    fn formats_hours_and_minutes() {
        assert_eq!(format_hm(0), "0:00");
        assert_eq!(format_hm(65), "1:05");
        assert_eq!(format_hm(570), "9:30");
    }

    #[test]
    fn rejects_overlapping_entries_same_day() {
        let entries = vec![
            entry("2026-08-18", "09:00", "12:15", "acme-corp"),
            entry("2026-08-18", "11:00", "13:00", "side-project"),
        ];
        let err = summarize(&entries).unwrap_err();
        assert!(err.contains("overlaps with"));
    }

    #[test]
    fn allows_back_to_back_entries() {
        let entries = vec![
            entry("2026-08-18", "09:00", "12:00", "acme-corp"),
            entry("2026-08-18", "12:00", "13:00", "side-project"),
        ];
        assert!(summarize(&entries).is_ok());
    }

    #[test]
    fn rejects_overlap_across_midnight_into_next_day() {
        let entries = vec![
            entry("2026-08-18", "22:00", "02:00", "acme-corp"),
            entry("2026-08-19", "01:00", "03:00", "side-project"),
        ];
        let err = summarize(&entries).unwrap_err();
        assert!(err.contains("overlaps with"));
    }

    #[test]
    fn detects_overlap_regardless_of_input_order() {
        let entries = vec![
            entry("2026-08-19", "10:00", "11:00", "side-project"),
            entry("2026-08-18", "09:00", "12:00", "acme-corp"),
            entry("2026-08-18", "10:00", "13:00", "other-corp"),
        ];
        let err = summarize(&entries).unwrap_err();
        assert!(err.contains("overlaps with"));
    }
}
