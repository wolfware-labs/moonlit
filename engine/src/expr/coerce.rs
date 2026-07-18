//! Scalar coercion (§5.4, `ToClrType` parity): raw config strings are typed in a fixed order —
//! `bool → i64 → f64 → datetime → String`. Datetime recognition is a deliberately narrow ISO-ish
//! set (RFC3339 with offset, `T`/space naive datetimes, and date-only), not .NET's liberal parse.

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime};

/// A coerced scalar. Datetimes carry a real `chrono` value so conditions can compare them.
#[derive(Clone, Debug, PartialEq)]
pub enum Scalar {
    Bool(bool),
    Int(i64),
    Float(f64),
    DateTime(DateTime<FixedOffset>),
    Str(String),
}

/// Coerce a raw string to a [`Scalar`] in the fixed §5.4 order.
pub fn coerce(raw: &str) -> Scalar {
    if raw.eq_ignore_ascii_case("true") {
        return Scalar::Bool(true);
    }
    if raw.eq_ignore_ascii_case("false") {
        return Scalar::Bool(false);
    }
    if let Ok(i) = raw.parse::<i64>() {
        return Scalar::Int(i);
    }
    if let Ok(f) = raw.parse::<f64>() {
        return Scalar::Float(f);
    }
    if let Some(dt) = parse_datetime(raw) {
        return Scalar::DateTime(dt);
    }
    Scalar::Str(raw.to_string())
}

fn parse_datetime(s: &str) -> Option<DateTime<FixedOffset>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt);
    }
    let utc = FixedOffset::east_opt(0)?;
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return ndt.and_local_timezone(utc).single();
    }
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return ndt.and_local_timezone(utc).single();
    }
    if let Ok(nd) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return nd.and_hms_opt(0, 0, 0)?.and_local_timezone(utc).single();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bool_is_case_insensitive() {
        assert_eq!(coerce("true"), Scalar::Bool(true));
        assert_eq!(coerce("FALSE"), Scalar::Bool(false));
        assert_eq!(coerce("True"), Scalar::Bool(true));
    }

    #[test]
    fn int_before_float() {
        assert_eq!(coerce("42"), Scalar::Int(42));
        assert_eq!(coerce("-7"), Scalar::Int(-7));
        assert_eq!(coerce("3.5"), Scalar::Float(3.5));
    }

    #[test]
    fn rfc3339_and_naive_and_date_only_coerce_to_datetime() {
        assert!(matches!(
            coerce("2024-01-02T03:04:05Z"),
            Scalar::DateTime(_)
        ));
        assert!(matches!(coerce("2024-01-02T03:04:05"), Scalar::DateTime(_)));
        assert!(matches!(coerce("2024-01-02 03:04:05"), Scalar::DateTime(_)));
        assert!(matches!(coerce("2024-01-02"), Scalar::DateTime(_)));
    }

    #[test]
    fn non_iso_dates_stay_strings() {
        assert_eq!(coerce("01/02/2024"), Scalar::Str("01/02/2024".to_string()));
        assert_eq!(coerce("main"), Scalar::Str("main".to_string()));
    }

    #[test]
    fn datetime_equality_round_trips_offset() {
        let a = coerce("2024-01-02T03:04:05+00:00");
        let b = coerce("2024-01-02T03:04:05Z");
        assert_eq!(a, b);
    }
}
