use aws_config::Region;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_cloudwatchlogs as cwl;
use chrono::{DateTime, Utc};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AwsLogError {
    #[error("Failed to create CloudWatch Logs client: {0}")]
    ClientInit(String),

    #[error("Failed to fetch log groups for region '{region}' and profile '{profile}': {source}")]
    FetchLogGroups {
        region: String,
        profile: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Failed to fetch log events for group '{group}': {source}")]
    FetchLogEvents {
        group: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Invalid time filter '{value}': {reason}")]
    TimeParse { value: String, reason: String },
}

#[derive(Debug)]
struct SimpleLogEvent<'a> {
    timestamp_ms: i64,
    message: &'a str,
}

pub async fn fetch_log_groups(region: &str, profile: &str) -> Result<Vec<String>, AwsLogError> {
    let client = build_cloudwatch_client(region, profile)
        .await
        .map_err(|e| AwsLogError::ClientInit(e.to_string()))?;

    let mut out = Vec::new();
    let mut next_token: Option<String> = None;

    loop {
        let mut req = client.describe_log_groups();
        if let Some(token) = &next_token {
            req = req.next_token(token);
        }

        let resp = req.send().await.map_err(|e| AwsLogError::FetchLogGroups {
            region: region.to_string(),
            profile: profile.to_string(),
            source: Box::new(e),
        })?;

        for g in resp.log_groups() {
            if let Some(name) = g.log_group_name() {
                out.push(name.to_string());
            }
        }

        next_token = resp.next_token().map(|s| s.to_string());
        if next_token.is_none() {
            break;
        }
    }

    out.sort();
    Ok(out)
}

pub async fn fetch_log_events(
    region: &str,
    profile: &str,
    log_group: &str,
    start: &str,
    end: &str,
    pattern: &str,
) -> Result<(Vec<String>, Option<i64>), AwsLogError> {
    let client = build_cloudwatch_client(region, profile)
        .await
        .map_err(|e| AwsLogError::ClientInit(e.to_string()))?;

    let now_ms = Utc::now().timestamp_millis();
    let start_ms = if start.trim().is_empty() {
        // default: last 15m
        now_ms - 15 * 60 * 1_000
    } else {
        parse_relative_or_absolute_ms(start, now_ms).map_err(|reason| AwsLogError::TimeParse {
            value: start.to_string(),
            reason,
        })?
    };

    let end_ms = if end.trim().is_empty() {
        now_ms
    } else {
        parse_relative_or_absolute_ms(end, now_ms).map_err(|reason| AwsLogError::TimeParse {
            value: end.to_string(),
            reason,
        })?
    };

    let mut out = Vec::new();
    let mut last_ts: Option<i64> = None;
    let mut next_token: Option<String> = None;

    // normalize the pattern once up-front
    let normalized_pattern = normalize_filter_pattern(pattern);

    loop {
        let mut req = client
            .filter_log_events()
            .log_group_name(log_group)
            .start_time(start_ms)
            .end_time(end_ms);

        if !normalized_pattern.trim().is_empty() {
            req = req.filter_pattern(&normalized_pattern);
        }
        if let Some(tok) = &next_token {
            req = req.next_token(tok);
        }

        let resp = req.send().await.map_err(|e| AwsLogError::FetchLogEvents {
            group: log_group.to_string(),
            source: Box::new(e),
        })?;

        for ev in resp.events() {
            let ts = ev.timestamp().unwrap_or(0);

            if let Some(current) = last_ts {
                if ts > current {
                    last_ts = Some(ts);
                }
            } else {
                last_ts = Some(ts);
            }
            let msg = ev.message().unwrap_or("");

            let simple = SimpleLogEvent {
                timestamp_ms: ts,
                message: msg,
            };

            out.push(format_log_event(&simple));
        }

        let new_token = resp.next_token().map(|s| s.to_string());
        if new_token.is_none() || new_token == next_token {
            break;
        }
        next_token = new_token;
    }

    Ok((out, last_ts))
}

fn format_log_event(ev: &SimpleLogEvent<'_>) -> String {
    let ts_str = match chrono::DateTime::<Utc>::from_timestamp_millis(ev.timestamp_ms) {
        Some(dt) => dt.to_rfc3339(),
        None => ev.timestamp_ms.to_string(),
    };

    let msg = ev.message.trim_end();

    if let Some((prefix, json)) = msg.split_once('{') {
        let json_with_brace = format!("{{{}", json);

        if let Some(pretty) = pretty_json_if_possible(&json_with_brace) {
            return format!("{}{}\n{}", ts_str, prefix, pretty);
        } else {
            return format!("{ts_str} {msg}");
        }
    }

    format!("{ts_str} {msg}")
}

fn parse_rfc3339_to_ms(s: &str) -> Result<i64, String> {
    let s = s.trim();

    if let Ok(dt) = s.parse::<DateTime<chrono::FixedOffset>>() {
        return Ok(dt.with_timezone(&Utc).timestamp_millis());
    }

    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        let dt = naive.and_utc();
        return Ok(dt.timestamp_millis());
    }

    Err(format!(
        "Invalid datetime '{s}'. Use either:\n\
         - RFC3339: 2025-12-11T10:00:00Z\n\
         - Simple:  2025-12-11 10:00:00"
    ))
}

fn parse_relative_or_absolute_ms(s: &str, now_ms: i64) -> Result<i64, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("empty time string".to_string());
    }

    // Relative syntax: -5m, -1h, -2d, -30s
    // Accept: optional leading '-', then number, then unit
    if trimmed.starts_with('-') {
        // strip leading '-'
        let rest = &trimmed[1..];
        // split into numeric prefix and unit suffix
        let (num_str, unit) = rest
            .chars()
            .enumerate()
            .find(|(_, c)| !c.is_ascii_digit())
            .map(|(idx, _)| rest.split_at(idx))
            .unwrap_or((rest, ""));

        if num_str.is_empty() {
            return Err(format!("Invalid relative time '{s}': missing number"));
        }
        let value: i64 = num_str
            .parse()
            .map_err(|_| format!("Invalid relative time '{s}': '{}' is not a number", num_str))?;

        let multiplier_ms: i64 = match unit {
            "s" | "" => 1_000,           // seconds (or default to seconds if no unit)
            "m" => 60 * 1_000,           // minutes
            "h" => 60 * 60 * 1_000,      // hours
            "d" => 24 * 60 * 60 * 1_000, // days
            _ => {
                return Err(format!(
                    "Invalid relative time unit in '{s}'. Use one of: s, m, h, d"
                ));
            }
        };

        let delta = value
            .checked_mul(multiplier_ms)
            .ok_or_else(|| format!("Relative time '{s}' is too large"))?;

        return Ok(now_ms - delta);
    }

    // Fallback: treat as absolute datetime (RFC3339 or "YYYY-MM-DD HH:MM:SS")
    parse_rfc3339_to_ms(trimmed)
}

fn pretty_json_if_possible(s: &str) -> Option<String> {
    let trimmed = s.trim_start();
    if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
        return None;
    }

    let v: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    serde_json::to_string_pretty(&v).ok()
}

fn normalize_filter_pattern(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // If it already looks like a CloudWatch filter expression, don't touch it.
    // Examples: "{ $.routing_id = 123 }", "ERROR", "[level = \"error\"]"
    if trimmed.starts_with('{') || trimmed.starts_with('[') || trimmed.contains('$') {
        return trimmed.to_string();
    }

    // Support multiple "field=value" or "field:value" pairs separated by whitespace:
    //   routing_id=123 task="batch-attendances"
    //     -> { $.routing_id = 123 && $.task = "batch-attendances" }
    //
    // If parsing fails, fall back to the original string.
    let mut conditions = Vec::new();

    for token in trimmed.split_whitespace() {
        if let Some((field, value)) = token.split_once('=') {
            let field = field.trim();
            let value = value.trim();
            if !field.is_empty() && !value.is_empty() {
                conditions.push(format!("$.{} = {}", field, value));
                continue;
            }
        } else if let Some((field, value)) = token.split_once(':') {
            let field = field.trim();
            let value = value.trim();
            if !field.is_empty() && !value.is_empty() {
                conditions.push(format!("$.{} = {}", field, value));
                continue;
            }
        }

        // If any token doesn't match our simple shorthand, bail out and
        // return the original pattern unchanged.
        return trimmed.to_string();
    }

    if conditions.is_empty() {
        // Fallback: leave as-is, so arbitrary patterns (e.g. "ERROR") still work.
        trimmed.to_string()
    } else {
        format!("{{ {} }}", conditions.join(" && "))
    }
}

async fn build_cloudwatch_client(region: &str, profile: &str) -> Result<cwl::Client, AwsLogError> {
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region.to_string())))
        .or_default_provider()
        .or_else(Region::new("eu-west-1"));

    let cfg = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(region_provider)
        .profile_name(profile)
        .load()
        .await;

    Ok(cwl::Client::new(&cfg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};
    use chrono::{TimeZone, Utc};

    #[test]
    fn parses_full_rfc3339() {
        let input = "2025-12-11T10:00:00Z";
        let ms = parse_rfc3339_to_ms(input).expect("should parse RFC3339");

        let dt = Utc
            .timestamp_millis_opt(ms)
            .single()
            .expect("valid timestamp");
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 11);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 0);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn parses_simple_datetime() {
        let input = "2025-12-11 10:00:00";
        let ms = parse_rfc3339_to_ms(input).expect("should parse simple form");

        let dt = Utc
            .timestamp_millis_opt(ms)
            .single()
            .expect("valid timestamp");
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 11);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 0);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn rejects_invalid_datetime() {
        let input = "not a datetime";
        let err = parse_rfc3339_to_ms(input).expect_err("should be invalid");
        assert!(err.contains("Invalid datetime"), "error message was: {err}");
    }

    #[test]
    fn pretty_json_formats_object() {
        let raw = r#"{ "a": 1, "b": "two" }"#;
        let pretty = pretty_json_if_possible(raw).expect("should pretty-print");
        // basic sanity: starts with '{' and contains newlines/indentation
        assert!(pretty.starts_with("{"));
        assert!(pretty.contains("\n"));
        assert!(pretty.contains("\"a\""));
        assert!(pretty.contains("\"b\""));
    }

    #[test]
    fn pretty_json_formats_array() {
        let raw = r#"[1, 2, 3]"#;
        let pretty = pretty_json_if_possible(raw).expect("should pretty-print");
        assert!(pretty.starts_with("["));
        assert!(pretty.contains("\n"));
        assert!(pretty.contains("1"));
        assert!(pretty.contains("3"));
    }

    #[test]
    fn pretty_json_handles_leading_prefix_trim() {
        // this simulates a log having spaces before the JSON
        let raw = "   {\"k\": \"v\"}";
        let pretty = pretty_json_if_possible(raw).expect("should pretty-print");
        assert!(pretty.contains("\"k\""));
        assert!(pretty.contains("\"v\""));
    }

    #[test]
    fn pretty_json_rejects_non_json() {
        let raw = "INFO something happened";
        assert!(pretty_json_if_possible(raw).is_none());
    }

    #[test]
    fn format_log_event_plain_message() {
        // 2025-01-01T00:00:00Z in millis
        let dt = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).single().unwrap();
        let ev = SimpleLogEvent {
            timestamp_ms: dt.timestamp_millis(),
            message: "INFO hello world",
        };

        let out = format_log_event(&ev);

        // Accept both Z and +00:00 forms
        assert!(
            out.contains("2025-01-01T00:00:00Z") || out.contains("2025-01-01T00:00:00+00:00"),
            "expected RFC3339 timestamp with UTC offset, got: {out}"
        );

        assert!(
            out.ends_with("INFO hello world"),
            "expected message at end, got: {out}"
        );
    }

    #[test]
    fn format_log_event_with_json_object_pretty_prints() {
        let dt = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).single().unwrap();
        let ev = SimpleLogEvent {
            timestamp_ms: dt.timestamp_millis(),
            message: "INFO {\"a\":1,\"b\":\"two\"}",
        };

        let out = format_log_event(&ev);
        assert!(out.contains("INFO "), "prefix should be kept, got: {out}");
        assert!(
            out.contains("\"a\""),
            "pretty JSON should contain key a, got: {out}"
        );
        assert!(
            out.contains("\n"),
            "pretty JSON should be multi-line, got: {out}"
        );
    }

    #[test]
    fn format_log_event_with_malformed_json_falls_back() {
        let dt = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).single().unwrap();
        // Missing closing brace → not valid JSON
        let ev = SimpleLogEvent {
            timestamp_ms: dt.timestamp_millis(),
            message: "INFO {\"a\":1",
        };

        let out = format_log_event(&ev);
        // In this case we should *not* pretty-print, just show the raw message
        assert!(
            !out.contains("\n{\"a\""),
            "should not contain pretty-printed JSON, got: {out}"
        );
        assert!(
            out.ends_with("INFO {\"a\":1"),
            "should fall back to 'ts message', got: {out}"
        );
    }

    #[test]
    fn normalize_filter_pattern_leaves_full_syntax_untouched() {
        let raw = "{ $.routing_id = 123 }";
        let norm = normalize_filter_pattern(raw);
        assert_eq!(norm, "{ $.routing_id = 123 }");
    }

    #[test]
    fn normalize_filter_pattern_parses_equals_shorthand() {
        let raw = "routing_id=123";
        let norm = normalize_filter_pattern(raw);
        assert_eq!(norm, "{ $.routing_id = 123 }");
    }

    #[test]
    fn normalize_filter_pattern_parses_colon_shorthand() {
        let raw = "routing_id:123";
        let norm = normalize_filter_pattern(raw);
        assert_eq!(norm, "{ $.routing_id = 123 }");
    }

    #[test]
    fn normalize_filter_pattern_leaves_simple_term() {
        let raw = "ERROR";
        let norm = normalize_filter_pattern(raw);
        assert_eq!(norm, "ERROR");
    }

    #[test]
    fn normalize_filter_pattern_with_string_value() {
        let raw = "level:error";
        let norm = normalize_filter_pattern(raw);
        assert_eq!(norm, "{ $.level = error }");
    }

    #[test]
    fn normalize_filter_pattern_multiple_equals_pairs() {
        let raw = "routing_id=1364 task=\"batch-attendances\"";
        let norm = normalize_filter_pattern(raw);
        assert_eq!(
            norm,
            "{ $.routing_id = 1364 && $.task = \"batch-attendances\" }"
        );
    }

    #[test]
    fn normalize_filter_pattern_multiple_colon_pairs() {
        let raw = "level:error env:prod";
        let norm = normalize_filter_pattern(raw);
        assert_eq!(norm, "{ $.level = error && $.env = prod }");
    }

    #[test]
    fn normalize_filter_pattern_mixed_pairs() {
        let raw = "routing_id=1364 task:\"batch-attendances\"";
        let norm = normalize_filter_pattern(raw);
        assert_eq!(
            norm,
            "{ $.routing_id = 1364 && $.task = \"batch-attendances\" }"
        );
    }

    #[test]
    fn normalize_filter_pattern_bails_out_on_unknown_token() {
        // Contains a token we don't understand ("foo"), so we should
        // return the original string unchanged.
        let raw = "routing_id=1364 foo";
        let norm = normalize_filter_pattern(raw);
        assert_eq!(norm, raw);
    }

    #[test]
    fn format_log_event_preserves_newlines_in_message() {
        let ev = SimpleLogEvent {
            timestamp_ms: 0,
            message: "line1\nline2\nline3",
        };

        let out = format_log_event(&ev);
        assert!(out.contains("line1"));
        assert!(out.contains("line2"));
        assert!(out.contains("line3"));
    }

    #[test]
    fn normalize_filter_pattern_empty_or_whitespace() {
        assert_eq!(normalize_filter_pattern(""), "");
        assert_eq!(normalize_filter_pattern("   "), "");
    }

    #[test]
    fn parse_relative_time_minutes() {
        let now = Utc.with_ymd_and_hms(2025, 1, 1, 0, 10, 0).single().unwrap();
        let now_ms = now.timestamp_millis();

        let ms = parse_relative_or_absolute_ms("-5m", now_ms).expect("should parse -5m");
        let dt = Utc.timestamp_millis_opt(ms).single().unwrap();

        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 1);
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 5);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn parse_relative_time_hours() {
        let now = Utc.with_ymd_and_hms(2025, 1, 1, 3, 0, 0).single().unwrap();
        let now_ms = now.timestamp_millis();

        let ms = parse_relative_or_absolute_ms("-1h", now_ms).expect("should parse -1h");
        let dt = Utc.timestamp_millis_opt(ms).single().unwrap();

        assert_eq!(dt.hour(), 2);
    }

    #[test]
    fn parse_relative_time_days() {
        let now = Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).single().unwrap();
        let now_ms = now.timestamp_millis();

        let ms = parse_relative_or_absolute_ms("-1d", now_ms).expect("should parse -1d");
        let dt = Utc.timestamp_millis_opt(ms).single().unwrap();

        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 1);
    }

    #[test]
    fn parse_relative_time_rejects_missing_number() {
        let now_ms = Utc::now().timestamp_millis();
        let err = parse_relative_or_absolute_ms("-", now_ms).expect_err("'-' should be invalid");
        assert!(
            err.contains("missing number"),
            "expected 'missing number' in error, got: {err}"
        );
    }

    #[test]
    fn parse_relative_time_rejects_invalid_unit() {
        let now_ms = Utc::now().timestamp_millis();
        let err =
            parse_relative_or_absolute_ms("-5x", now_ms).expect_err("'-5x' should be invalid");
        assert!(
            err.contains("Invalid relative time unit"),
            "expected unit error, got: {err}"
        );
    }

    #[test]
    fn parse_relative_time_falls_back_to_absolute() {
        let input = "2025-12-11T10:00:00Z";
        let now_ms = 0; // doesn't matter; absolute path ignores it
        let ms = parse_relative_or_absolute_ms(input, now_ms).expect("should parse absolute");
        let dt = Utc.timestamp_millis_opt(ms).single().unwrap();

        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 11);
        assert_eq!(dt.hour(), 10);
    }

    #[test]
    fn aws_log_error_timeparse_includes_value_and_reason() {
        let err = AwsLogError::TimeParse {
            value: "-5x".to_string(),
            reason: "Invalid relative time unit in '-5x'. Use one of: s, m, h, d".to_string(),
        };

        let msg = format!("{err}");
        assert!(
            msg.contains("-5x") && msg.contains("Invalid relative time unit"),
            "expected value and reason in AwsLogError::TimeParse display, got: {msg}"
        );
    }

    #[test]
    fn normalize_filter_pattern_leaves_explicit_expressions_unchanged() {
        let input = r#"{ $.routing_id = 123 && $.task = "foo" }"#;
        let out = normalize_filter_pattern(input);
        assert_eq!(out, input);
    }

    #[test]
    fn normalize_filter_pattern_builds_shorthand_expression() {
        let input = r#"routing_id=123 task="batch-attendances""#;
        let out = normalize_filter_pattern(input);
        assert_eq!(
            out,
            r#"{ $.routing_id = 123 && $.task = "batch-attendances" }"#
        );
    }

    #[test]
    fn normalize_filter_pattern_bails_out_on_mixed_tokens() {
        let input = "routing_id=123 weird-token";
        let out = normalize_filter_pattern(input);
        // Should fall back to the original string when it can't interpret tokens
        assert_eq!(out, input);
    }
}
