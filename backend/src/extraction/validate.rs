//! Code-side checks on what the model read. The model only transcribes; every
//! arithmetic relationship is verified here, and anything doubtful is flagged
//! so the app can highlight it on the review screen.

use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, NaiveDateTime, TimeZone};

use super::schema::{CATEGORY_SLUGS, Extraction, Issue};
use crate::time::jakarta;

/// Stores often round the total to the nearest Rp 100 ("pembulatan" / donation).
pub const SUM_TOLERANCE: i64 = 100;
/// Totals below this are almost always a misread separator ("25.000" read as 25).
pub const SUSPICIOUS_TOTAL: i64 = 500;

#[derive(Debug, Clone)]
pub struct Validated {
    pub extraction: Extraction,
    pub issues: Vec<Issue>,
    pub occurred_at: Option<DateTime<FixedOffset>>,
}

impl Validated {
    pub fn needs_review(&self) -> bool {
        !self.issues.is_empty()
    }
}

pub fn validate(mut e: Extraction, now: DateTime<FixedOffset>, threshold: f32) -> Validated {
    let mut issues = Vec::new();

    if !e.is_receipt {
        issues.push(Issue::new(
            "image",
            "not_a_receipt",
            "This doesn't look like a receipt.",
        ));
    }

    // --- merchant ---------------------------------------------------------
    if e.merchant.value.is_none() {
        issues.push(Issue::new("merchant", "missing", "Store name not found."));
        e.merchant.confidence = 0.0;
    } else if e.merchant.confidence < threshold {
        issues.push(Issue::new(
            "merchant",
            "low_confidence",
            "Store name is hard to read.",
        ));
    }

    // --- date -------------------------------------------------------------
    let occurred_at = e.date.value.as_deref().and_then(parse_receipt_date);
    match occurred_at {
        None => {
            if e.date.value.is_some() {
                issues.push(Issue::new(
                    "date",
                    "unparseable",
                    "Date couldn't be understood.",
                ));
            } else {
                issues.push(Issue::new("date", "missing", "Date not found."));
            }
            e.date.confidence = 0.0;
        }
        Some(dt) => {
            e.date.value = Some(dt.to_rfc3339());
            if dt > now + Duration::days(1) {
                issues.push(Issue::new("date", "in_future", "Date is in the future."));
                e.date.confidence = e.date.confidence.min(0.3);
            } else if dt < now - Duration::days(366) {
                issues.push(Issue::new(
                    "date",
                    "too_old",
                    "Date is more than a year ago.",
                ));
                e.date.confidence = e.date.confidence.min(0.3);
            } else if e.date.confidence < threshold {
                issues.push(Issue::new(
                    "date",
                    "low_confidence",
                    "Date is hard to read.",
                ));
            }
        }
    }

    // --- items ------------------------------------------------------------
    for (i, item) in e.items.iter_mut().enumerate() {
        let field = format!("items[{i}]");
        item.flagged = false;
        if item.name.trim().is_empty() {
            item.flagged = true;
            issues.push(Issue::new(&field, "missing_name", "Item name not found."));
        }
        if item.total <= 0 {
            item.flagged = true;
            issues.push(Issue::new(
                &field,
                "bad_amount",
                "Item amount is missing or negative.",
            ));
        } else if let Some(unit) = item.unit_price {
            let expected = (item.qty * unit as f64).round() as i64;
            if (expected - item.total).abs() > 1 {
                item.flagged = true;
                item.confidence = item.confidence.min(0.4);
                issues.push(Issue::new(
                    &field,
                    "item_math",
                    format!(
                        "Quantity × price doesn't match the line amount for {}.",
                        item.name
                    ),
                ));
            }
        }
        if !item.flagged && item.confidence < threshold {
            item.flagged = true;
            issues.push(Issue::new(
                &field,
                "low_confidence",
                format!("{} is hard to read.", item.name),
            ));
        }
    }

    // --- totals -----------------------------------------------------------
    let discount = e.discount.value.map(i64::abs).unwrap_or(0);
    let tax = e.tax.value.unwrap_or(0).max(0);
    let service = e.service.value.unwrap_or(0).max(0);
    let items_sum: i64 = e.items.iter().map(|i| i.total.max(0)).sum();
    let has_items = !e.items.is_empty();

    if let (Some(subtotal), true) = (e.subtotal.value, has_items)
        && (subtotal - items_sum).abs() > SUM_TOLERANCE
    {
        e.subtotal.confidence = e.subtotal.confidence.min(0.5);
        issues.push(Issue::new(
            "subtotal",
            "subtotal_mismatch",
            "Items don't add up to the subtotal. Some lines may be missing or misread.",
        ));
    }

    match e.total.value {
        None => {
            if has_items {
                // Code (not the model) computes a fallback, clearly marked as a guess.
                e.total.value = Some(items_sum - discount + tax + service);
                e.total.confidence = 0.3;
                issues.push(Issue::new(
                    "total",
                    "missing",
                    "Total not found; calculated from the items.",
                ));
            } else {
                e.total.confidence = 0.0;
                issues.push(Issue::new("total", "missing", "Total not found."));
            }
        }
        Some(total) => {
            if total < SUSPICIOUS_TOTAL {
                e.total.confidence = e.total.confidence.min(0.3);
                issues.push(Issue::new(
                    "total",
                    "amount_suspicious",
                    "Total looks too small. Check the thousands separator.",
                ));
            } else if has_items {
                // PPN can be included in item prices or added on top; accept either.
                let candidates = [items_sum - discount, items_sum - discount + tax + service];
                if !candidates
                    .iter()
                    .any(|c| (c - total).abs() <= SUM_TOLERANCE)
                {
                    e.total.confidence = e.total.confidence.min(0.5);
                    issues.push(Issue::new(
                        "total",
                        "sum_mismatch",
                        "Items don't add up to the total. Check the highlighted lines.",
                    ));
                }
            }
            if e.total.confidence < threshold && !issues.iter().any(|i| i.field == "total") {
                issues.push(Issue::new(
                    "total",
                    "low_confidence",
                    "Total is hard to read.",
                ));
            }
        }
    }

    // --- category ---------------------------------------------------------
    let known = e
        .category_suggestion
        .value
        .as_deref()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| CATEGORY_SLUGS.contains(&v.as_str()));
    match known {
        Some(slug) => e.category_suggestion.value = Some(slug),
        None => {
            e.category_suggestion.value = Some("other".into());
            e.category_suggestion.confidence = 0.0;
        }
    }

    Validated {
        extraction: e,
        issues,
        occurred_at,
    }
}

/// Accepts RFC 3339, naive ISO date-times (assumed WIB), date-only ISO, and
/// day-first Indonesian formats (15/10/2026, 15-10-26).
pub fn parse_receipt_date(raw: &str) -> Option<DateTime<FixedOffset>> {
    let s = raw.trim();
    let tz = jakarta();
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&tz));
    }
    for fmt in [
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
    ] {
        if let Ok(n) = NaiveDateTime::parse_from_str(s, fmt) {
            return tz.from_local_datetime(&n).single();
        }
    }
    for fmt in [
        "%Y-%m-%d", "%d/%m/%Y", "%d-%m-%Y", "%d.%m.%Y", "%d/%m/%y", "%d-%m-%y",
    ] {
        // "%Y" happily reads "26" as year 26; skip those so "%y" gets its turn.
        if let Ok(d) = NaiveDate::parse_from_str(s, fmt)
            && d.year() >= 2000
        {
            // Date only: use midday so the transaction never slips into a neighbouring day or month.
            return tz.from_local_datetime(&d.and_hms_opt(12, 0, 0)?).single();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn now() -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339("2026-10-15T20:00:00+07:00").unwrap()
    }

    fn parse(v: serde_json::Value) -> Extraction {
        serde_json::from_value(v).unwrap()
    }

    fn clean_receipt() -> serde_json::Value {
        json!({
            "is_receipt": true,
            "merchant": { "value": "Indomaret", "confidence": 0.97 },
            "date": { "value": "2026-10-15T19:05:00+07:00", "confidence": 0.9 },
            "items": [
                { "name": "Indomie Goreng", "qty": 5, "unit_price": 3500, "total": 17500, "confidence": 0.95 },
                { "name": "UHT milk 1 L", "qty": 1, "unit_price": 19900, "total": 19900, "confidence": 0.95 },
                { "name": "White bread", "qty": 1, "unit_price": 16500, "total": 16500, "confidence": 0.95 },
                { "name": "Eggs, 10 pack", "qty": 1, "unit_price": 27000, "total": 27000, "confidence": 0.9 },
                { "name": "Dish soap", "qty": 1, "unit_price": 6500, "total": 6500, "confidence": 0.9 }
            ],
            "subtotal": { "value": 87400, "confidence": 0.9 },
            "tax": { "value": null, "confidence": 0 },
            "service": { "value": null, "confidence": 0 },
            "discount": { "value": null, "confidence": 0 },
            "total": { "value": 87400, "confidence": 0.98 },
            "category_suggestion": { "value": "groceries", "confidence": 0.8 },
            "currency": "IDR"
        })
    }

    #[test]
    fn clean_receipt_has_no_issues() {
        let v = validate(parse(clean_receipt()), now(), 0.75);
        assert!(v.issues.is_empty(), "{:?}", v.issues);
        assert!(!v.needs_review());
        assert_eq!(
            v.occurred_at.unwrap().to_rfc3339(),
            "2026-10-15T19:05:00+07:00"
        );
    }

    #[test]
    fn line_math_mismatch_flags_item() {
        let mut r = clean_receipt();
        r["items"][3]["total"] = json!(72000);
        let v = validate(parse(r), now(), 0.75);
        assert!(v.extraction.items[3].flagged);
        assert!(
            v.issues
                .iter()
                .any(|i| i.code == "item_math" && i.field == "items[3]")
        );
        // Item sum no longer matches the printed total either.
        assert!(v.issues.iter().any(|i| i.code == "sum_mismatch"));
    }

    #[test]
    fn total_mistaken_for_cash_paid_is_caught() {
        let mut r = clean_receipt();
        r["total"]["value"] = json!(100000); // TUNAI line instead of TOTAL
        let v = validate(parse(r), now(), 0.75);
        assert!(v.issues.iter().any(|i| i.code == "sum_mismatch"));
        assert!(v.extraction.total.confidence <= 0.5);
    }

    #[test]
    fn tax_on_top_and_discount_are_accepted() {
        let mut r = clean_receipt();
        r["discount"]["value"] = json!(7400);
        r["tax"]["value"] = json!(8000);
        r["total"]["value"] = json!(88000);
        r["subtotal"]["value"] = json!(null);
        let v = validate(parse(r), now(), 0.75);
        assert!(v.issues.is_empty(), "{:?}", v.issues);
    }

    #[test]
    fn rounding_within_tolerance_is_fine() {
        let mut r = clean_receipt();
        r["total"]["value"] = json!(87500);
        let v = validate(parse(r), now(), 0.75);
        assert!(v.issues.is_empty(), "{:?}", v.issues);
    }

    #[test]
    fn misread_separator_is_suspicious() {
        let mut r = clean_receipt();
        r["items"] = json!([]);
        r["subtotal"]["value"] = json!(null);
        r["total"]["value"] = json!(87.4);
        let v = validate(parse(r), now(), 0.75);
        assert!(v.issues.iter().any(|i| i.code == "amount_suspicious"));
    }

    #[test]
    fn missing_total_is_computed_by_code_with_low_confidence() {
        let mut r = clean_receipt();
        r["total"]["value"] = json!(null);
        let v = validate(parse(r), now(), 0.75);
        assert_eq!(v.extraction.total.value, Some(87_400));
        assert!(v.extraction.total.confidence <= 0.3);
        assert!(
            v.issues
                .iter()
                .any(|i| i.field == "total" && i.code == "missing")
        );
    }

    #[test]
    fn future_and_unknown_category() {
        let mut r = clean_receipt();
        r["date"]["value"] = json!("2027-01-01");
        r["category_suggestion"]["value"] = json!("snacks");
        let v = validate(parse(r), now(), 0.75);
        assert!(v.issues.iter().any(|i| i.code == "in_future"));
        assert_eq!(
            v.extraction.category_suggestion.value.as_deref(),
            Some("other")
        );
        assert_eq!(v.extraction.category_suggestion.confidence, 0.0);
    }

    #[test]
    fn low_confidence_items_are_flagged() {
        let mut r = clean_receipt();
        r["items"][4]["confidence"] = json!(0.3);
        let v = validate(parse(r), now(), 0.75);
        assert!(v.extraction.items[4].flagged);
        assert!(
            v.issues
                .iter()
                .any(|i| i.field == "items[4]" && i.code == "low_confidence")
        );
    }

    #[test]
    fn not_a_receipt() {
        let v = validate(parse(json!({ "is_receipt": false })), now(), 0.75);
        assert!(v.issues.iter().any(|i| i.code == "not_a_receipt"));
    }

    #[test]
    fn date_formats() {
        let f = |s| parse_receipt_date(s).map(|d| d.to_rfc3339());
        assert_eq!(
            f("2026-10-15T19:05:00+07:00").as_deref(),
            Some("2026-10-15T19:05:00+07:00")
        );
        assert_eq!(
            f("2026-10-15T12:05:00Z").as_deref(),
            Some("2026-10-15T19:05:00+07:00")
        );
        assert_eq!(
            f("2026-10-15 19:05").as_deref(),
            Some("2026-10-15T19:05:00+07:00")
        );
        assert_eq!(f("05/10/26").as_deref(), Some("2026-10-05T12:00:00+07:00"));
        assert_eq!(
            f("15-10-2026").as_deref(),
            Some("2026-10-15T12:00:00+07:00")
        );
        assert_eq!(f("kemarin"), None);
    }
}
