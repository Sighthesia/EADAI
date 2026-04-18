use crate::cli::ParserKind;
use crate::message::{ParserMeta, ParserStatus};
use crate::serial::{FrameStatus, FramedLine};

/// Parses one framed serial line with the configured parser strategy.
///
/// - `parser_kind`: selected parser strategy.
/// - `line`: framed serial line, including framing status.
pub fn parse_framed_line(parser_kind: ParserKind, line: &FramedLine) -> ParserMeta {
    if line.status == FrameStatus::Overflow {
        return ParserMeta::malformed(None, "frame overflow");
    }

    match parser_kind {
        ParserKind::Auto => {
            let key_value = crate::key_value_parser::parse_line(&line.payload.text);
            if key_value.status != ParserStatus::Unparsed {
                return key_value;
            }

            crate::measurement_parser::parse_line(&line.payload.text)
        }
        ParserKind::Measurements => crate::measurement_parser::parse_line(&line.payload.text),
        ParserKind::KeyValue => crate::key_value_parser::parse_line(&line.payload.text),
        ParserKind::Bmi088 => ParserMeta::unparsed(),
    }
}
