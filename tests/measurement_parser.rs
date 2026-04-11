use eadai::measurement_parser;
use eadai::message::ParserStatus;

#[test]
fn parses_single_value_with_embedded_unit() {
    let parser = measurement_parser::parse_line("CH1=1.23V OK");

    assert_eq!(parser.parser_name.as_deref(), Some("measurements"));
    assert_eq!(parser.status, ParserStatus::Parsed);
    assert_eq!(
        parser.fields.get("channel_id").map(String::as_str),
        Some("CH1")
    );
    assert_eq!(
        parser.fields.get("value").map(String::as_str),
        Some("1.23V")
    );
    assert_eq!(
        parser.fields.get("numeric_value").map(String::as_str),
        Some("1.23")
    );
    assert_eq!(parser.fields.get("unit").map(String::as_str), Some("V"));
    assert_eq!(parser.fields.get("status").map(String::as_str), Some("OK"));
}

#[test]
fn parses_single_value_with_separate_unit_and_status() {
    let parser = measurement_parser::parse_line("CH1=1.23 V OK");

    assert_eq!(parser.status, ParserStatus::Parsed);
    assert_eq!(parser.fields.get("unit").map(String::as_str), Some("V"));
    assert_eq!(parser.fields.get("status").map(String::as_str), Some("OK"));
}

#[test]
fn parses_multiple_measurement_fields() {
    let parser = measurement_parser::parse_line("ts=1712800000 ax=1 ay=2 az=3");

    assert_eq!(parser.status, ParserStatus::Parsed);
    assert_eq!(
        parser.fields.get("timestamp").map(String::as_str),
        Some("1712800000")
    );
    assert_eq!(
        parser.fields.get("channel_id").map(String::as_str),
        Some("ax")
    );
    assert_eq!(parser.fields.get("field.ax").map(String::as_str), Some("1"));
    assert_eq!(parser.fields.get("field.ay").map(String::as_str), Some("2"));
    assert_eq!(parser.fields.get("field.az").map(String::as_str), Some("3"));
    assert_eq!(
        parser.fields.get("field_count").map(String::as_str),
        Some("4")
    );
}

#[test]
fn keeps_noise_lines_unparsed() {
    let parser = measurement_parser::parse_line("boot complete");

    assert!(parser.parser_name.is_none());
    assert_eq!(parser.status, ParserStatus::Unparsed);
    assert!(parser.fields.is_empty());
}

#[test]
fn marks_broken_pairs_as_malformed() {
    let parser = measurement_parser::parse_line("temp= WARN");

    assert_eq!(parser.parser_name.as_deref(), Some("measurements"));
    assert_eq!(parser.status, ParserStatus::Malformed);
    assert_eq!(
        parser.fields.get("error").map(String::as_str),
        Some("missing key or value")
    );
}
