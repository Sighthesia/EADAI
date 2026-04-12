use crate::message::ParserMeta;
use std::collections::BTreeMap;

const MEASUREMENT_PARSER_NAME: &str = "measurements";

/// Parses common text telemetry lines into normalized parser metadata.
///
/// - `line`: line text without trailing newline bytes.
pub fn parse_line(line: &str) -> ParserMeta {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return ParserMeta::unparsed();
    }

    let tokens = trimmed
        .split(|character: char| character == ',' || character.is_whitespace())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    let mut pairs = Vec::new();
    let mut pending_unit = None;
    let mut pending_status = None;

    for token in tokens {
        if let Some((key, value)) = split_pair(token) {
            if key.is_empty() || value.is_empty() {
                return ParserMeta::malformed(
                    Some(MEASUREMENT_PARSER_NAME),
                    "missing key or value",
                );
            }

            if !is_valid_key(key) {
                return ParserMeta::malformed(Some(MEASUREMENT_PARSER_NAME), "invalid field name");
            }

            pairs.push((key.to_string(), value.to_string()));
            continue;
        }

        if pairs.is_empty() {
            continue;
        }

        if pairs.len() == 1
            && pending_unit.is_none()
            && !is_status_token(token)
            && is_unit_token(token)
        {
            pending_unit = Some(token.to_string());
            continue;
        }

        if pending_status.is_none() && is_status_token(token) {
            pending_status = Some(token.to_string());
            continue;
        }

        return ParserMeta::malformed(Some(MEASUREMENT_PARSER_NAME), "unexpected trailing token");
    }

    if pairs.is_empty() {
        return ParserMeta::unparsed();
    }

    let mut fields = BTreeMap::new();
    fields.insert("field_count".to_string(), pairs.len().to_string());

    if let Some(status) = pending_status {
        fields.insert("status".to_string(), status);
    }

    for (key, value) in &pairs {
        fields.insert(format!("field.{key}"), value.clone());
        if is_timestamp_key(key) {
            fields.insert("timestamp".to_string(), value.clone());
        }
    }

    let (primary_key, primary_value) = select_primary_pair(&pairs);
    let (numeric_value, embedded_unit) = split_numeric_value(primary_value);

    fields.insert("channel_id".to_string(), primary_key.to_string());
    fields.insert("value".to_string(), primary_value.to_string());

    if let Some(value) = numeric_value {
        fields.insert("numeric_value".to_string(), value);
    }

    if let Some(unit) = pending_unit.or(embedded_unit) {
        fields.insert("unit".to_string(), unit);
    }

    ParserMeta::parsed(MEASUREMENT_PARSER_NAME, fields)
}

fn split_pair(token: &str) -> Option<(&str, &str)> {
    token.split_once('=').or_else(|| token.split_once(':'))
}

fn is_valid_key(value: &str) -> bool {
    value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
}

fn is_unit_token(token: &str) -> bool {
    token.chars().all(|character| {
        character.is_ascii_alphabetic() || matches!(character, '%' | '/' | '_' | '-')
    })
}

fn is_status_token(token: &str) -> bool {
    matches!(
        token,
        "OK" | "WARN" | "ERR" | "ERROR" | "FAIL" | "FAILED" | "ALARM"
    )
}

fn is_timestamp_key(key: &str) -> bool {
    matches!(key, "ts" | "time" | "timestamp")
}

fn select_primary_pair(pairs: &[(String, String)]) -> (&str, &str) {
    pairs
        .iter()
        .find(|(key, _)| !is_timestamp_key(key))
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .unwrap_or_else(|| (pairs[0].0.as_str(), pairs[0].1.as_str()))
}

fn split_numeric_value(value: &str) -> (Option<String>, Option<String>) {
    let boundaries = value
        .char_indices()
        .map(|(index, _)| index)
        .skip(1)
        .chain(std::iter::once(value.len()));
    let mut numeric_end = None;

    for boundary in boundaries {
        if value[..boundary].parse::<f64>().is_ok() {
            numeric_end = Some(boundary);
        }
    }

    let Some(boundary) = numeric_end else {
        return (None, None);
    };

    let suffix = value[boundary..].trim();
    if suffix.is_empty() {
        return (Some(value[..boundary].to_string()), None);
    }

    if is_unit_token(suffix) {
        return (
            Some(value[..boundary].to_string()),
            Some(suffix.to_string()),
        );
    }

    (None, None)
}
