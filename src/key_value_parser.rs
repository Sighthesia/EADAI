use crate::message::ParserMeta;

const KEY_VALUE_PARSER_NAME: &str = "key_value";

/// Parses `key:value` lines into normalized parser metadata.
///
/// - `line`: line text without trailing newline bytes.
pub fn parse_line(line: &str) -> ParserMeta {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return ParserMeta::unparsed();
    }

    let Some((channel_id, value)) = trimmed.split_once(':') else {
        return ParserMeta::unparsed();
    };

    let channel_id = channel_id.trim();
    let value = value.trim();

    if channel_id.is_empty() || value.is_empty() {
        return ParserMeta::malformed(Some(KEY_VALUE_PARSER_NAME), "missing key or value");
    }

    if !is_valid_key(channel_id) {
        return ParserMeta::malformed(Some(KEY_VALUE_PARSER_NAME), "invalid channel id");
    }

    let mut fields = std::collections::BTreeMap::new();
    fields.insert("channel_id".to_string(), channel_id.to_string());
    fields.insert("value".to_string(), value.to_string());

    ParserMeta::parsed(KEY_VALUE_PARSER_NAME, fields)
}

fn is_valid_key(value: &str) -> bool {
    value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
}
