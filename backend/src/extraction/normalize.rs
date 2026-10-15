//! Normalising the numbers LLMs return. Models are told to output integer
//! rupiah, but they still sometimes return "Rp 25.000" or 25000.0, so every
//! money field goes through these lenient deserializers.

use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// Parse an Indonesian (or occasionally English) formatted amount into whole rupiah.
///
/// Rules:
/// - "Rp", "IDR", spaces and trailing "-" are ignored.
/// - With both `.` and `,`, the last one is the decimal separator ("25.000,50" / "25,000.50").
/// - With only one kind of separator, groups of exactly three digits mean thousands
///   ("25.000", "1.250.000", "25,000"); otherwise it is a decimal ("25,5", "3.5").
/// - Parentheses or a leading `-` make it negative (discount lines).
pub fn parse_idr(raw: &str) -> Option<i64> {
    let mut s = raw.trim().to_string();
    let mut negative = false;
    if s.starts_with('(') && s.ends_with(')') {
        negative = true;
        s = s[1..s.len() - 1].to_string();
    }
    let upper = s.to_uppercase();
    let mut s: String = upper
        .replace("RP.", "")
        .replace("RP", "")
        .replace("IDR", "")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if let Some(rest) = s.strip_prefix('-') {
        negative = !negative;
        s = rest.to_string();
    }
    let s = s.trim_end_matches(['-', ',', '.']);
    if s.is_empty()
        || !s
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == ',')
    {
        return None;
    }

    let last_dot = s.rfind('.');
    let last_comma = s.rfind(',');
    let (int_part, frac_part): (String, String) = match (last_dot, last_comma) {
        (Some(d), Some(c)) => {
            let dec = d.max(c);
            let int: String = s[..dec].chars().filter(|c| c.is_ascii_digit()).collect();
            (int, s[dec + 1..].to_string())
        }
        (Some(_), None) => split_single(s, '.')?,
        (None, Some(_)) => split_single(s, ',')?,
        (None, None) => (s.to_string(), String::new()),
    };

    let int: i64 = if int_part.is_empty() {
        0
    } else {
        int_part.parse().ok()?
    };
    // Round half up on the first fractional digit; rupiah has no practical cents.
    let round_up = frac_part.chars().next().is_some_and(|c| c >= '5');
    let value = int + i64::from(round_up);
    Some(if negative { -value } else { value })
}

fn split_single(s: &str, sep: char) -> Option<(String, String)> {
    let groups: Vec<&str> = s.split(sep).collect();
    let thousands = groups.len() > 1
        && !groups[0].is_empty()
        && groups[0].len() <= 3
        && groups[1..].iter().all(|g| g.len() == 3);
    if thousands {
        Some((groups.concat(), String::new()))
    } else if groups.len() == 2 {
        Some((groups[0].to_string(), groups[1].to_string()))
    } else {
        None
    }
}

/// Convert a JSON value (number, string, or null) to whole rupiah.
pub fn money_from_value(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64().or_else(|| {
            n.as_f64()
                .filter(|f| f.is_finite())
                .map(|f| f.round() as i64)
        }),
        Value::String(s) => parse_idr(s),
        _ => None,
    }
}

pub fn de_money_opt<'de, D: Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
    let v = Value::deserialize(d)?;
    Ok(money_from_value(&v))
}

pub fn de_money_or_zero<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    de_money_opt(d).map(|v| v.unwrap_or(0))
}

/// Quantities may be fractional (weighed produce), and may come as "0,512".
pub fn de_qty<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    let v = Value::deserialize(d)?;
    let q = match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().replace(',', ".").parse::<f64>().ok(),
        _ => None,
    };
    Ok(q.filter(|q| q.is_finite() && *q > 0.0).unwrap_or(1.0))
}

/// Confidence clamped to [0, 1]; missing or malformed means "no confidence".
pub fn de_confidence<'de, D: Deserializer<'de>>(d: D) -> Result<f32, D::Error> {
    let v = Value::deserialize(d)?;
    let c = match v {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::String(s) => s.trim().parse().unwrap_or(0.0),
        _ => 0.0,
    };
    Ok(if c.is_finite() {
        c.clamp(0.0, 1.0) as f32
    } else {
        0.0
    })
}

pub fn de_string_opt<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let v = Value::deserialize(d)?;
    Ok(match v {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indonesian_thousands() {
        assert_eq!(parse_idr("Rp 25.000"), Some(25_000));
        assert_eq!(parse_idr("Rp. 1.250.000"), Some(1_250_000));
        assert_eq!(parse_idr("IDR 87.400"), Some(87_400));
        assert_eq!(parse_idr("25.000,00"), Some(25_000));
        assert_eq!(parse_idr("12.345,50"), Some(12_346));
    }

    #[test]
    fn english_format_and_plain() {
        assert_eq!(parse_idr("25,000"), Some(25_000));
        assert_eq!(parse_idr("1,250,000.00"), Some(1_250_000));
        assert_eq!(parse_idr("87400"), Some(87_400));
    }

    #[test]
    fn decimals_and_negatives() {
        assert_eq!(parse_idr("3,5"), Some(4));
        assert_eq!(parse_idr("-5.000"), Some(-5_000));
        assert_eq!(parse_idr("(2.500)"), Some(-2_500));
        assert_eq!(parse_idr("Rp 15.000,-"), Some(15_000));
    }

    #[test]
    fn garbage_is_none() {
        assert_eq!(parse_idr(""), None);
        assert_eq!(parse_idr("TOTAL"), None);
        assert_eq!(parse_idr("1.2.3"), None);
    }

    #[test]
    fn json_values() {
        assert_eq!(money_from_value(&serde_json::json!(25000)), Some(25_000));
        assert_eq!(money_from_value(&serde_json::json!(25000.4)), Some(25_000));
        assert_eq!(
            money_from_value(&serde_json::json!("Rp 25.000")),
            Some(25_000)
        );
        assert_eq!(money_from_value(&serde_json::json!(null)), None);
    }
}
