// Extracted from [tower-lsp](https://github.com/ebkalderon/tower-lsp).
// Copyright (c) 2020 Eyal Kalderon. MIT License.
// See codec.rs.

use std::str;

use nom::{
    branch::alt,
    bytes::streaming::{is_not, tag},
    character::streaming::{char, crlf, digit1, space0},
    combinator::{map_res, opt},
    multi::length_data,
    sequence::{delimited, terminated, tuple},
    IResult,
};

// Get JSON message from input using the Content-Length header.
pub fn parse_message(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let content_len = delimited(tag("Content-Length: "), digit1, crlf);

    let utf8 = alt((tag("utf-8"), tag("utf8")));
    let charset = tuple((char(';'), space0, tag("charset="), utf8));
    let content_type = tuple((tag("Content-Type: "), is_not(";\r"), opt(charset), crlf));

    let header = terminated(terminated(content_len, opt(content_type)), crlf);

    let header = map_res(header, |s: &[u8]| str::from_utf8(s));
    let length = map_res(header, |s: &str| s.parse::<usize>());
    let message = length_data(length);

    message(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_exact() {
        let decoded =
            r#"{"jsonrpc":"2.0","method":"initialize","id":1,"params":{"capabilities":{}}}"#;
        let sample = format!("Content-Length: {}\r\n\r\n{}", decoded.len(), decoded);
        assert_eq!(
            parse_message(sample.as_bytes()),
            Ok(("".as_bytes(), decoded.as_bytes()))
        );
    }

    #[test]
    fn test_optional_content_type() {
        let decoded =
            r#"{"jsonrpc":"2.0","method":"initialize","id":1,"params":{"capabilities":{}}}"#;
        let content_type = "Content-Type: application/vscode-jsonrpc; charset=utf-8".to_string();

        let sample = format!(
            "Content-Length: {}\r\n{}\r\n\r\n{}",
            decoded.len(),
            content_type,
            decoded
        );
        assert_eq!(
            parse_message(sample.as_bytes()),
            Ok(("".as_bytes(), decoded.as_bytes()))
        );
    }

    #[test]
    fn test_incomplete_error_with_size() {
        let decoded =
            r#"{"jsonrpc":"2.0","method":"initialize","id":1,"params":{"capabilities":{}}}"#;

        let sample = format!("Content-Length: {}\r\n\r\n", decoded.len());
        assert_eq!(
            parse_message(sample.as_bytes()),
            Err(nom::Err::Incomplete(nom::Needed::Size(decoded.len())))
        );

        assert_eq!(
            parse_message((sample + "{").as_bytes()),
            Err(nom::Err::Incomplete(nom::Needed::Size(decoded.len())))
        );
    }
}
