use aws_config::Region;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_cloudwatchlogs as cwl;
use chrono::{DateTime, Utc};

#[derive(Debug)]
struct SimpleLogEvent<'a> {
    timestamp_ms: i64,
    message: &'a str,
}

pub async fn fetch_log_groups(region: &str, profile: &str) -> Result<Vec<String>, cwl::Error> {
    let region_provider = RegionProviderChain::first_try(Some(Region::new(region.to_string())))
        .or_default_provider()
        .or_else(Region::new("eu-west-1"));

    let cfg = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(region_provider)
        .profile_name(profile)
        .load()
        .await;

    let client = cwl::Client::new(&cfg);

    let mut out = Vec::new();
    let mut next_token: Option<String> = None;

    loop {
        let mut req = client.describe_log_groups();
        if let Some(token) = &next_token {
            req = req.next_token(token);
        }

        let resp = req.send().await?;

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
) -> Result<Vec<String>, String> {
    let cfg = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(Region::new(region.to_string()))
        .profile_name(profile)
        .load()
        .await;

    let client = cwl::Client::new(&cfg);

    let now_ms = Utc::now().timestamp_millis();
    let start_ms = if start.trim().is_empty() {
        now_ms - 15 * 60 * 1000
    } else {
        parse_rfc3339_to_ms(start)?
    };

    let end_ms = if end.trim().is_empty() {
        now_ms
    } else {
        parse_rfc3339_to_ms(end)?
    };

    let mut out = Vec::new();
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

        let resp = req.send().await.map_err(|e| e.to_string())?;

        for ev in resp.events() {
            let ts = ev.timestamp().unwrap_or(0);
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

    Ok(out)
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
    if trimmed.starts_with('{')
        || trimmed.starts_with('[')
        || trimmed.contains(' ')
        || trimmed.contains('$')
    {
        return trimmed.to_string();
    }

    // Very simple "field=value" or "field:value" shorthand:
    //   routing_id=123 -> { $.routing_id = 123 }
    //   routing_id:123 -> { $.routing_id = 123 }
    if let Some((field, value)) = trimmed.split_once('=') {
        let field = field.trim();
        let value = value.trim();
        if !field.is_empty() && !value.is_empty() {
            return format!("{{ $.{} = {} }}", field, value);
        }
    } else if let Some((field, value)) = trimmed.split_once(':') {
        let field = field.trim();
        let value = value.trim();
        if !field.is_empty() && !value.is_empty() {
            return format!("{{ $.{} = {} }}", field, value);
        }
    }

    // Fallback: leave as-is, so arbitrary patterns (e.g. "ERROR") still work.
    trimmed.to_string()
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
}
