//! Time helpers. Indonesia (WIB) has no DST, so a fixed +07:00 offset is exact.

use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, TimeZone, Utc};

pub fn jakarta() -> FixedOffset {
    FixedOffset::east_opt(7 * 3600).expect("valid offset")
}

pub fn now_jakarta() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&jakarta())
}

/// First day of the month containing `date`.
pub fn month_start(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).expect("day 1 exists")
}

/// First day of the following month.
pub fn next_month(month: NaiveDate) -> NaiveDate {
    let (y, m) = if month.month() == 12 {
        (month.year() + 1, 1)
    } else {
        (month.year(), month.month() + 1)
    };
    NaiveDate::from_ymd_opt(y, m, 1).expect("day 1 exists")
}

pub fn days_in_month(month: NaiveDate) -> u32 {
    (next_month(month) - month_start(month)).num_days() as u32
}

/// Parse "2026-10" into the first day of that month.
pub fn parse_month(raw: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(&format!("{raw}-01"), "%Y-%m-%d").ok()
}

/// UTC bounds [start, end) of a calendar month in Jakarta time.
pub fn month_bounds_utc(month: NaiveDate) -> (DateTime<Utc>, DateTime<Utc>) {
    let tz = jakarta();
    let start = tz
        .from_local_datetime(&month_start(month).and_hms_opt(0, 0, 0).expect("midnight"))
        .single()
        .expect("fixed offset is unambiguous");
    let end = tz
        .from_local_datetime(&next_month(month).and_hms_opt(0, 0, 0).expect("midnight"))
        .single()
        .expect("fixed offset is unambiguous");
    (start.with_timezone(&Utc), end.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_math() {
        let oct = parse_month("2026-10").unwrap();
        assert_eq!(days_in_month(oct), 31);
        assert_eq!(
            next_month(NaiveDate::from_ymd_opt(2026, 12, 1).unwrap()).year(),
            2027
        );
        assert_eq!(days_in_month(parse_month("2028-02").unwrap()), 29);
        assert!(parse_month("2026-13").is_none());
    }

    #[test]
    fn bounds_are_shifted_to_utc() {
        let (start, _) = month_bounds_utc(parse_month("2026-10").unwrap());
        assert_eq!(start.to_rfc3339(), "2026-09-30T17:00:00+00:00");
    }
}
