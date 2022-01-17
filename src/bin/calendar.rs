use std::fs::read_to_string;

use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone, Utc};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
struct RawEvent {
    start: Option<DateTime<FixedOffset>>,
    end: Option<DateTime<FixedOffset>>,
    summary: Option<String>,
}
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let ical = read_to_string(&args[1])?;

    let mut events = vec![];

    let mut event_started = false;
    let mut start = None;
    let mut end = None;
    let mut summary: Option<String> = None;
    for line in ical.split("\n").map(|s| s.trim()) {
        match line {
            "BEGIN:VEVENT" => {
                event_started = true;
            }
            "END:VEVENT" => {
                assert!(event_started);
                let event = RawEvent {
                    start: start.take(),
                    end: end.take(),
                    summary: summary.take(),
                };
                events.push(event);
                event_started = false;
            }
            _ => {
                if !event_started {
                    continue;
                }
                if line.starts_with("DTSTART:") {
                    start = Some(parse_date_time(&line[8..])?);
                } else if line.starts_with("DTEND:") {
                    end = Some(parse_date_time(&line[6..])?);
                } else if line.starts_with("SUMMARY:") {
                    summary = Some((&line[8..]).to_string());
                }
            }
        }
    }

    let threshold = DateTime::parse_from_rfc3339("2022-01-10T00:00:00+09:00")?;

    let mut events: Vec<_> = events
        .into_iter()
        .filter_map(|raw_event| {
            let start = raw_event.start?;
            let end = raw_event.end?;
            let summary = raw_event.summary?;
            Some((start, end, summary))
        })
        .collect();
    events.sort();
    for (start, end, summary) in events {
        if start < threshold {
            continue;
        }
        println!(
            "{}\t{}\t{}",
            start.format("%Y/%m/%d %H:%M"),
            end.format("%Y/%m/%d %H:%M"),
            summary
        );
    }

    Ok(())
}

fn parse_date_time(s: &str) -> Result<DateTime<FixedOffset>> {
    let year = s[0..4].parse()?;
    let month = s[4..6].parse()?;
    let day = s[6..8].parse()?;

    let hour = s[9..11].parse()?;
    let min = s[11..13].parse()?;

    let dt = NaiveDate::from_ymd(year, month, day).and_hms(hour, min, 0);
    let utc: DateTime<Utc> = DateTime::from_utc(dt, Utc);
    let jst = FixedOffset::east(9 * 3600);

    Ok(utc.with_timezone(&jst))
}
