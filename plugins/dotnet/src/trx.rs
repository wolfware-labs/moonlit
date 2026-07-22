//! Parse the single `<Counters .../>` element of a VSTest `.trx` results file.

use regex::Regex;

#[derive(Debug, PartialEq, Eq)]
pub struct TrxCounters {
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub total: u32,
}

/// Extract counts from the TRX `<Counters .../>` element. `None` when absent.
/// `skipped = total - executed` (VSTest counts skipped tests as not-executed).
pub fn parse_counters(xml: &str) -> Option<TrxCounters> {
    let counters_re = Regex::new(r"<Counters\b[^>]*>").ok()?;
    let el = counters_re.find(xml)?.as_str();
    let attr = |name: &str| -> u32 {
        Regex::new(&format!(r#"{name}="(\d+)""#))
            .ok()
            .and_then(|re| re.captures(el))
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0)
    };
    let total = attr("total");
    let executed = attr("executed");
    Some(TrxCounters {
        passed: attr("passed"),
        failed: attr("failed"),
        skipped: total.saturating_sub(executed),
        total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRX: &str = r#"<?xml version="1.0"?><TestRun><ResultSummary outcome="Completed">
      <Counters total="10" executed="8" passed="7" failed="1" error="0" timeout="0"
        aborted="0" inconclusive="0" notExecuted="2" /></ResultSummary></TestRun>"#;

    #[test]
    fn parses_counts_and_skipped_is_total_minus_executed() {
        let c = parse_counters(TRX).unwrap();
        assert_eq!(
            c,
            TrxCounters {
                passed: 7,
                failed: 1,
                skipped: 2,
                total: 10
            }
        );
    }

    #[test]
    fn all_pass_zero_skipped() {
        let xml = r#"<Counters total="3" executed="3" passed="3" failed="0" />"#;
        assert_eq!(
            parse_counters(xml).unwrap(),
            TrxCounters {
                passed: 3,
                failed: 0,
                skipped: 0,
                total: 3
            }
        );
    }

    #[test]
    fn zero_tests() {
        let xml = r#"<Counters total="0" executed="0" passed="0" failed="0" />"#;
        assert_eq!(
            parse_counters(xml).unwrap(),
            TrxCounters {
                passed: 0,
                failed: 0,
                skipped: 0,
                total: 0
            }
        );
    }

    #[test]
    fn missing_counters_element_returns_none() {
        assert_eq!(parse_counters("<TestRun></TestRun>"), None);
    }
}
