use crate::message::ParserMeta;

const KEY_VALUE_PARSER_NAME: &str = "key_value";

/// Parses `key:value` lines into normalized parser metadata.
///
/// - `line`: line text without trailing newline bytes.
pub fn parse_line(line: &str) -> ParserMeta {
    let trimmed = line.trim();
    let Some((channel_id, value)) = trimmed.split_once(':') else {
        return ParserMeta::default();
    };

    let channel_id = channel_id.trim();
    let value = value.trim();

    if channel_id.is_empty() || value.is_empty() {
        return ParserMeta::default();
    }

    let mut fields = std::collections::BTreeMap::new();
    fields.insert("channel_id".to_string(), channel_id.to_string());
    fields.insert("value".to_string(), value.to_string());

    ParserMeta {
        parser_name: Some(KEY_VALUE_PARSER_NAME.to_string()),
        fields,
    }
}
