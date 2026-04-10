use eadai::key_value_parser;

#[test]
fn parses_key_value_lines_into_normalized_fields() {
    let parser = key_value_parser::parse_line("imu_pitch:1.25");

    assert_eq!(parser.parser_name.as_deref(), Some("key_value"));
    assert_eq!(
        parser.fields.get("channel_id").map(String::as_str),
        Some("imu_pitch")
    );
    assert_eq!(parser.fields.get("value").map(String::as_str), Some("1.25"));
}

#[test]
fn ignores_non_matching_lines() {
    let parser = key_value_parser::parse_line("boot complete");

    assert!(parser.parser_name.is_none());
    assert!(parser.fields.is_empty());
}

#[test]
fn trims_whitespace_around_channel_and_value() {
    let parser = key_value_parser::parse_line("  temp  :  42  ");

    assert_eq!(
        parser.fields.get("channel_id").map(String::as_str),
        Some("temp")
    );
    assert_eq!(parser.fields.get("value").map(String::as_str), Some("42"));
}
