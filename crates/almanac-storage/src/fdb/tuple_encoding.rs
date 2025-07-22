//! Tuple encoding utilities for FoundationDB
//!
//! This module provides encoding and decoding utilities for storing
//! structured data as tuples in FoundationDB.

#![allow(dead_code)]

use bytes::{Buf, BufMut, Bytes, BytesMut};

/// Encode a value into a tuple element
pub trait TupleEncode {
    fn encode(&self, buf: &mut BytesMut);
}

/// Decode a value from tuple bytes
pub trait TupleDecode: Sized {
    fn decode(buf: &mut Bytes) -> Result<Self, TupleError>;
}

/// Tuple encoding/decoding errors
#[derive(Debug, thiserror::Error)]
pub enum TupleError {
    #[error("Invalid tuple format")]
    InvalidFormat,

    #[error("Unexpected end of data")]
    UnexpectedEnd,

    #[error("Invalid type marker: {0}")]
    InvalidTypeMarker(u8),

    #[error("UTF-8 decoding error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
}

// Type markers for tuple encoding
const TYPE_NULL: u8 = 0x00;
const TYPE_BYTES: u8 = 0x01;
const TYPE_STRING: u8 = 0x02;
const TYPE_INT: u8 = 0x03;
const TYPE_UINT: u8 = 0x04;

impl TupleEncode for String {
    fn encode(&self, buf: &mut BytesMut) {
        buf.extend_from_slice(&[TYPE_STRING]);
        buf.extend_from_slice(self.as_bytes());
        buf.extend_from_slice(&[0x00]); // Null terminator
    }
}

impl TupleEncode for &str {
    fn encode(&self, buf: &mut BytesMut) {
        buf.extend_from_slice(&[TYPE_STRING]);
        buf.extend_from_slice(self.as_bytes());
        buf.extend_from_slice(&[0x00]); // Null terminator
    }
}

impl TupleEncode for u64 {
    fn encode(&self, buf: &mut BytesMut) {
        buf.extend_from_slice(&[TYPE_UINT]);
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl TupleEncode for i64 {
    fn encode(&self, buf: &mut BytesMut) {
        buf.extend_from_slice(&[TYPE_INT]);
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl TupleEncode for Vec<u8> {
    fn encode(&self, buf: &mut BytesMut) {
        buf.extend_from_slice(&[TYPE_BYTES]);
        // Implement proper escaping for FDB tuple format
        // Escape 0x00 bytes by replacing them with 0x00 0xFF
        for &byte in self {
            if byte == 0x00 {
                buf.extend_from_slice(&[0x00, 0xFF]);
            } else {
                buf.put_u8(byte);
            }
        }
        buf.extend_from_slice(&[0x00]);
    }
}

// Additional type constants
const TYPE_U64: u8 = 0x15;

impl TupleDecode for String {
    fn decode(buf: &mut Bytes) -> Result<Self, TupleError> {
        if buf.is_empty() || buf[0] != TYPE_STRING {
            return Err(TupleError::InvalidTypeMarker(buf[0]));
        }
        buf.advance(1); // Skip type marker

        // Find null terminator
        let null_pos = buf
            .iter()
            .position(|&b| b == 0x00)
            .ok_or(TupleError::UnexpectedEnd)?;

        let string_bytes = &buf[..null_pos];
        let result = std::str::from_utf8(string_bytes)?.to_string();
        buf.advance(null_pos + 1); // Skip past null terminator
        Ok(result)
    }
}

impl TupleDecode for u64 {
    fn decode(buf: &mut Bytes) -> Result<Self, TupleError> {
        if buf.len() < 9 || buf[0] != TYPE_U64 {
            return Err(TupleError::InvalidTypeMarker(buf[0]));
        }
        buf.advance(1); // Skip type marker

        if buf.len() < 8 {
            return Err(TupleError::UnexpectedEnd);
        }

        Ok(buf.get_u64())
    }
}

impl TupleDecode for Vec<u8> {
    fn decode(buf: &mut Bytes) -> Result<Self, TupleError> {
        if buf.is_empty() || buf[0] != TYPE_BYTES {
            return Err(TupleError::InvalidTypeMarker(buf[0]));
        }
        buf.advance(1); // Skip type marker

        let mut result = Vec::new();
        let mut i = 0;

        while i < buf.len() {
            if buf[i] == 0x00 {
                if i + 1 < buf.len() && buf[i + 1] == 0xFF {
                    // Escaped null byte
                    result.push(0x00);
                    i += 2;
                } else {
                    // End of bytes
                    buf.advance(i + 1);
                    return Ok(result);
                }
            } else {
                result.push(buf[i]);
                i += 1;
            }
        }

        Err(TupleError::UnexpectedEnd)
    }
}
