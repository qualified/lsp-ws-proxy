//! Error types defined by the JSON-RPC specification.
// Extracted from [tower-lsp](https://github.com/ebkalderon/tower-lsp).
// Copyright (c) 2020 Eyal Kalderon. MIT License.
// Changes:
// - removed methods to create Error

use std::fmt::{self, Display, Formatter};

use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A list of numeric error codes used in JSON-RPC responses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCode {
    /// Invalid JSON was received by the server.
    ParseError,
    /// The JSON sent is not a valid Request object.
    InvalidRequest,
    /// The method does not exist / is not available.
    MethodNotFound,
    /// Invalid method parameter(s).
    InvalidParams,
    /// Internal JSON-RPC error.
    InternalError,
    /// Reserved for implementation-defined server errors.
    ServerError(i64),

    /// The request was cancelled by the client.
    ///
    /// # Compatibility
    ///
    /// This error code is defined by the Language Server Protocol.
    RequestCancelled,
    /// The request was invalidated by another incoming request.
    ///
    /// # Compatibility
    ///
    /// This error code is specific to the Language Server Protocol.
    ContentModified,
}

impl ErrorCode {
    /// Returns the integer error code value.
    #[inline]
    pub fn code(&self) -> i64 {
        match *self {
            ErrorCode::ParseError => -32700,
            ErrorCode::InvalidRequest => -32600,
            ErrorCode::MethodNotFound => -32601,
            ErrorCode::InvalidParams => -32602,
            ErrorCode::InternalError => -32603,
            ErrorCode::RequestCancelled => -32800,
            ErrorCode::ContentModified => -32801,
            ErrorCode::ServerError(code) => code,
        }
    }

    /// Returns a human-readable description of the error.
    #[inline]
    pub fn description(&self) -> &'static str {
        match *self {
            ErrorCode::ParseError => "Parse error",
            ErrorCode::InvalidRequest => "Invalid request",
            ErrorCode::MethodNotFound => "Method not found",
            ErrorCode::InvalidParams => "Invalid params",
            ErrorCode::InternalError => "Internal error",
            ErrorCode::RequestCancelled => "Canceled",
            ErrorCode::ContentModified => "Content modified",
            ErrorCode::ServerError(_) => "Server error",
        }
    }
}

impl From<i64> for ErrorCode {
    #[inline]
    fn from(code: i64) -> Self {
        match code {
            -32700 => ErrorCode::ParseError,
            -32600 => ErrorCode::InvalidRequest,
            -32601 => ErrorCode::MethodNotFound,
            -32602 => ErrorCode::InvalidParams,
            -32603 => ErrorCode::InternalError,
            -32800 => ErrorCode::RequestCancelled,
            -32801 => ErrorCode::ContentModified,
            code => ErrorCode::ServerError(code),
        }
    }
}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.code(), f)
    }
}

impl<'a> Deserialize<'a> for ErrorCode {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        let code: i64 = Deserialize::deserialize(deserializer)?;
        Ok(ErrorCode::from(code))
    }
}

impl Serialize for ErrorCode {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.code().serialize(serializer)
    }
}

/// A JSON-RPC error object.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Error {
    /// A number indicating the error type that occurred.
    pub code: ErrorCode,
    /// A short description of the error.
    pub message: String,
    /// Additional information about the error, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.code.description(), self.message)
    }
}

impl std::error::Error for Error {}
