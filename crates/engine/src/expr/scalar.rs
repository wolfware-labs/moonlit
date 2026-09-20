use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime};

#[derive(Clone, Debug, PartialEq)]
pub enum Scalar {
    Bool(bool),
    Int(i64),
    Float(f64),
    DateTime(DateTime<FixedOffset>),
    Str(String),
}

impl From<&str> for Scalar {
    fn from(value: &str) -> Self {
        if value.eq_ignore_ascii_case("true") {
            return Scalar::Bool(true);
        }
        if value.eq_ignore_ascii_case("false") {
            return Scalar::Bool(false);
        }
        if let Ok(i) = value.parse::<i64>() {
            return Scalar::Int(i);
        }
        if let Ok(f) = value.parse::<f64>() {
            return Scalar::Float(f);
        }
        if let Some(dt) = parse_datetime(value) {
            return Scalar::DateTime(dt);
        }
        Scalar::Str(value.to_string())
    }
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
        assert_eq!("true".into(), Scalar::Bool(true));
        assert_eq!("FALSE".into(), Scalar::Bool(false));
        assert_eq!("True".into(), Scalar::Bool(true));
    }

    #[test]
    fn int_before_float() {
        assert_eq!("42".into(), Scalar::Int(42));
        assert_eq!("-7".into(), Scalar::Int(-7));
        assert_eq!("3.5".into(), Scalar::Float(3.5));
    }

    #[test]
    fn rfc3339_and_naive_and_date_only_coerce_to_datetime() {
        let expected_dt = DateTime::parse_from_rfc3339("2024-01-02T03:04:05Z").unwrap();
        assert_eq!("2024-01-02T03:04:05Z".into(), Scalar::DateTime(expected_dt));
        assert_eq!("2024-01-02T03:04:05".into(), Scalar::DateTime(expected_dt));
        assert_eq!("2024-01-02 03:04:05".into(), Scalar::DateTime(expected_dt));

        let expected_date = DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z").unwrap();
        assert_eq!("2024-01-02".into(), Scalar::DateTime(expected_date));
    }

    #[test]
    fn non_iso_dates_stay_strings() {
        assert_eq!("01/02/2024".into(), Scalar::Str("01/02/2024".to_string()));
        assert_eq!("main".into(), Scalar::Str("main".to_string()));
    }

    #[test]
    fn datetime_equality_round_trips_offset() {
        let a = "2024-01-02T03:04:05+00:00".into();
        let b = "2024-01-02T03:04:05Z".into();
        assert_eq!(a, b);
    }
}
