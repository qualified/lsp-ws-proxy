// Codec for LSP JSON RPC frame.
// Based on LanguageServerCodec from [tower-lsp](https://github.com/ebkalderon/tower-lsp).
// Copyright (c) 2020 Eyal Kalderon. MIT License.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io::{Error as IoError, Write},
    str::{self, Utf8Error},
};

use bytes::{Buf, BufMut, BytesMut};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Decoder, Encoder, FramedRead, FramedWrite};

use super::parser;

pub fn reader<R: AsyncRead>(inner: R) -> FramedRead<R, LspFrameCodec> {
    FramedRead::new(inner, LspFrameCodec::default())
}

pub fn writer<W: AsyncWrite>(inner: W) -> FramedWrite<W, LspFrameCodec> {
    FramedWrite::new(inner, LspFrameCodec::default())
}

/// Errors from LspFrameCodec.
#[derive(Debug)]
pub enum CodecError {
    /// The frame lacks the required `Content-Length` header.
    MissingHeader,
    /// The length value in the `Content-Length` header is invalid.
    InvalidLength,
    /// The media type in the `Content-Type` header is invalid.
    InvalidType,
    /// Failed to encode the frame.
    Encode(IoError),
    /// The frame contains invalid UTF8.
    Utf8(Utf8Error),
}

impl Display for CodecError {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match *self {
            Self::MissingHeader => write!(fmt, "missing required `Content-Length` header"),
            Self::InvalidLength => write!(fmt, "unable to parse content length"),
            Self::InvalidType => write!(fmt, "unable to parse content type"),
            Self::Encode(ref e) => write!(fmt, "failed to encode frame: {}", e),
            Self::Utf8(ref e) => write!(fmt, "frame contains invalid UTF8: {}", e),
        }
    }
}

impl Error for CodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            Self::Encode(ref e) => Some(e),
            Self::Utf8(ref e) => Some(e),
            _ => None,
        }
    }
}

impl From<IoError> for CodecError {
    fn from(error: IoError) -> Self {
        Self::Encode(error)
    }
}

impl From<Utf8Error> for CodecError {
    fn from(error: Utf8Error) -> Self {
        Self::Utf8(error)
    }
}

#[derive(Clone, Debug, Default)]
pub struct LspFrameCodec {
    remaining_bytes: usize,
}

impl Encoder<String> for LspFrameCodec {
    type Error = CodecError;
    fn encode(&mut self, item: String, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if !item.is_empty() {
            // `Content-Length: ` + `\r\n\r\n` = 20
            dst.reserve(item.len() + number_of_digits(item.len()) + 20);
            let mut writer = dst.writer();
            write!(writer, "Content-Length: {}\r\n\r\n{}", item.len(), item)?;
            writer.flush()?;
        }
        Ok(())
    }
}

impl Decoder for LspFrameCodec {
    type Item = String;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if self.remaining_bytes > src.len() {
            return Ok(None);
        }

        match parser::parse_message(src) {
            Ok((remaining, message)) => {
                let message = str::from_utf8(message)?.to_string();
                let len = src.len() - remaining.len();
                src.advance(len);
                self.remaining_bytes = 0;
                // Ignore empty frame
                if message.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(message))
                }
            }

            Err(nom::Err::Incomplete(nom::Needed::Size(needed))) => {
                self.remaining_bytes = needed.get();
                Ok(None)
            }

            Err(nom::Err::Incomplete(nom::Needed::Unknown)) => Ok(None),

            Err(nom::Err::Error(err)) | Err(nom::Err::Failure(err)) => {
                let code = err.code;
                let parsed_bytes = src.len() - err.input.len();
                src.advance(parsed_bytes);
                match parser::find_next_message(src) {
                    Ok((_, position)) => src.advance(position),
                    Err(_) => src.advance(src.len()),
                }
                match code {
                    nom::error::ErrorKind::Digit | nom::error::ErrorKind::MapRes => {
                        Err(CodecError::InvalidLength)
                    }
                    nom::error::ErrorKind::Char | nom::error::ErrorKind::IsNot => {
                        Err(CodecError::InvalidType)
                    }
                    _ => Err(CodecError::MissingHeader),
                }
            }
        }
    }
}

#[inline]
fn number_of_digits(mut n: usize) -> usize {
    let mut num_digits = 0;
    while n > 0 {
        n /= 10;
        num_digits += 1;
    }
    num_digits
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;

    use super::*;

    #[test]
    fn encode_and_decode() {
        let decoded = r#"{"jsonrpc":"2.0","method":"exit"}"#.to_string();
        let encoded = format!("Content-Length: {}\r\n\r\n{}", decoded.len(), decoded);

        let mut codec = LspFrameCodec::default();
        let mut buffer = BytesMut::new();
        codec.encode(decoded.clone(), &mut buffer).unwrap();
        assert_eq!(buffer, BytesMut::from(encoded.as_str()));

        let mut buffer = BytesMut::from(encoded.as_str());
        let message = codec.decode(&mut buffer).unwrap();
        assert_eq!(message, Some(decoded));
    }

    #[test]
    fn skips_encoding_empty_message() {
        let mut codec = LspFrameCodec::default();
        let mut buffer = BytesMut::new();
        codec.encode("".to_string(), &mut buffer).unwrap();
        assert_eq!(buffer, BytesMut::new());
    }

    #[test]
    fn decodes_optional_content_type() {
        let decoded = r#"{"jsonrpc":"2.0","method":"exit"}"#.to_string();
        let content_len = format!("Content-Length: {}", decoded.len());
        let content_type = "Content-Type: application/vscode-jsonrpc; charset=utf-8".to_string();
        let encoded = format!("{}\r\n{}\r\n\r\n{}", content_len, content_type, decoded);

        let mut codec = LspFrameCodec::default();
        let mut buffer = BytesMut::from(encoded.as_str());
        let message = codec.decode(&mut buffer).unwrap();
        assert_eq!(message, Some(decoded));
    }

    #[test]
    fn recovers_from_parse_error() {
        let decoded = r#"{"jsonrpc":"2.0","method":"exit"}"#.to_string();
        let encoded = format!("Content-Length: {}\r\n\r\n{}", decoded.len(), decoded);
        let mixed = format!("1234567890abcdefgh{}", encoded);

        let mut codec = LspFrameCodec::default();
        let mut buffer = BytesMut::from(mixed.as_str());

        match codec.decode(&mut buffer) {
            Err(CodecError::MissingHeader) => {}
            other => panic!("expected `Err(ParseError::MissingHeader)`, got {:?}", other),
        }

        let message = codec.decode(&mut buffer).unwrap();
        assert_eq!(message, Some(decoded));
    }
}
