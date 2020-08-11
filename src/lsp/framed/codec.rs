// Codec for LSP JSON RPC frame.
// Based on LanguageServerCodec from [tower-lsp](https://github.com/ebkalderon/tower-lsp).
// Copyright (c) 2020 Eyal Kalderon. MIT License.
// Ported to futures_codec.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{Error as IoError, Write};
use std::str::{self, Utf8Error};

use bytes::{buf::BufMutExt, Buf, BytesMut};
use futures_codec::{Decoder, Encoder, FramedRead, FramedWrite};
use futures_io::{AsyncRead, AsyncWrite};

use super::parser;

pub(crate) fn reader<R: AsyncRead>(inner: R) -> FramedRead<R, LspFrameCodec> {
    FramedRead::new(inner, LspFrameCodec::default())
}

pub(crate) fn writer<W: AsyncWrite>(inner: W) -> FramedWrite<W, LspFrameCodec> {
    FramedWrite::new(inner, LspFrameCodec::default())
}

/// Errors from LspFrameCodec.
#[derive(Debug)]
pub(crate) enum CodecError {
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
            CodecError::MissingHeader => write!(fmt, "missing required `Content-Length` header"),
            CodecError::InvalidLength => write!(fmt, "unable to parse content length"),
            CodecError::InvalidType => write!(fmt, "unable to parse content type"),
            CodecError::Encode(ref e) => write!(fmt, "failed to encode frame: {}", e),
            CodecError::Utf8(ref e) => write!(fmt, "frame contains invalid UTF8: {}", e),
        }
    }
}

impl Error for CodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            CodecError::Encode(ref e) => Some(e),
            CodecError::Utf8(ref e) => Some(e),
            _ => None,
        }
    }
}

impl From<IoError> for CodecError {
    fn from(error: IoError) -> Self {
        CodecError::Encode(error)
    }
}

impl From<Utf8Error> for CodecError {
    fn from(error: Utf8Error) -> Self {
        CodecError::Utf8(error)
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct LspFrameCodec {
    remaining_bytes: usize,
}

impl Encoder for LspFrameCodec {
    type Item = String;
    type Error = CodecError;
    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
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
        use nom::error::ErrorKind as NomErrorKind;
        use nom::Err::{Error as NomError, Failure as NomFailure, Incomplete};
        use nom::Needed;

        if self.remaining_bytes > src.len() {
            return Ok(None);
        }

        match parser::parse_message(&src) {
            Ok((remaining, message)) => {
                let message = str::from_utf8(message)?.to_string();
                let len = src.len() - remaining.len();
                src.advance(len);
                self.remaining_bytes = 0;

                Ok(Some(message))
            }

            Err(Incomplete(Needed::Size(needed))) => {
                self.remaining_bytes = needed;
                Ok(None)
            }

            Err(Incomplete(Needed::Unknown)) => Ok(None),

            Err(NomError((_, err))) | Err(NomFailure((_, err))) => loop {
                // To prevent infinite loop, advance the cursor until the buffer is empty or
                // the cursor reaches the next valid message.
                use CodecError::*;
                match parser::parse_message(&src) {
                    Err(_) if !src.is_empty() => src.advance(1),
                    _ => match err {
                        NomErrorKind::Digit | NomErrorKind::MapRes => return Err(InvalidLength),
                        NomErrorKind::Char | NomErrorKind::IsNot => return Err(InvalidType),
                        _ => return Err(CodecError::MissingHeader),
                    },
                }
            },
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
