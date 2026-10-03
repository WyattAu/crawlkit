//! Transport-encoding negotiation and decoding for fetched responses.
//!
//! # Why this exists
//!
//! Two independent facts combined to make every crawled page report
//! "large response not compressed":
//!
//! 1. `reqwest` built without the `gzip`/`brotli`/`deflate` features does not
//!    send an `Accept-Encoding` header at all. A server has no reason to
//!    compress a response that never advertised support, so it replied
//!    uncompressed and omitted `Content-Encoding` — making the site look
//!    uncompressed when a real browser (which does advertise support) received
//!    a `br`-compressed body at 28 KB instead of 71 KB.
//! 2. Turning those features on does not fix it either. reqwest then strips
//!    `Content-Encoding` (and `Content-Length`) from the response as it decodes,
//!    so a genuinely compressed response becomes indistinguishable from an
//!    uncompressed one at the point the analyzer inspects it.
//!
//! So crawlkit negotiates encoding explicitly and decodes explicitly: the wire
//! header stays observable, the decoded body is what analyzers parse, and the
//! transfer size is reported honestly.
//!
//! # Measured behaviour
//!
//! Against the site that produced the false positives, with an explicit
//! `Accept-Encoding: gzip, br, deflate`:
//!
//! ```text
//! curl -H 'Accept-Encoding: gzip, deflate, br' -I  ->  content-encoding: br  (27996 bytes)
//! crawlkit before this change                        ->  no header,   70714 bytes
//! ```

use std::io::Read;

/// Encoding token advertised to servers.
pub const ACCEPT_ENCODING: &str = "gzip, br, deflate";

/// The transfer encoding a response arrived with, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferEncoding {
    /// No `Content-Encoding`: the body arrived as sent.
    Identity,
    /// `gzip` (RFC 1952).
    Gzip,
    /// `deflate` (RFC 1951 / zlib wrapper).
    Deflate,
    /// `br` (Brotli, RFC 7932).
    Brotli,
    /// An encoding crawlkit did not ask for and therefore cannot decode.
    Unsupported,
}

impl TransferEncoding {
    /// Parse a `Content-Encoding` header value.
    ///
    /// Handles a list (`gzip, br`), casing, and whitespace. A response may in
    /// principle be encoded more than once; the last encoding applied is the
    /// one that must be undone first, so the list is read right-to-left.
    #[must_use]
    pub fn parse(header: Option<&str>) -> Self {
        let Some(raw) = header else {
            return Self::Identity;
        };
        let mut result = Self::Identity;
        // Encodings are listed in the order they were applied, so the *last*
        // entry is the outermost layer and must be undone first. Walking the
        // list left to right and keeping the final assignment yields exactly
        // that.
        for token in raw.split(',') {
            match token.trim().to_ascii_lowercase().as_str() {
                "" | "identity" => {}
                "gzip" | "x-gzip" => result = Self::Gzip,
                "deflate" => result = Self::Deflate,
                "br" => result = Self::Brotli,
                _ => result = Self::Unsupported,
            }
        }
        result
    }

    /// Whether the body had to be transformed to be read.
    #[must_use]
    pub fn is_encoded(self) -> bool {
        !matches!(self, Self::Identity)
    }
}

/// Decode a response body according to its transfer encoding.
///
/// Returns the decoded bytes. An encoding crawlkit did not negotiate, or a body
/// that fails to decode, is returned unchanged so that a decode failure degrades
/// into "the analyzer sees what the server sent" rather than an empty page.
#[must_use]
pub fn decode(body: &[u8], encoding: TransferEncoding) -> Vec<u8> {
    match encoding {
        TransferEncoding::Identity | TransferEncoding::Unsupported => body.to_vec(),
        TransferEncoding::Gzip => gunzip(body),
        TransferEncoding::Deflate => inflate(body),
        TransferEncoding::Brotli => brotli_decompress(body),
    }
}

fn gunzip(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    if flate2::read::GzDecoder::new(body)
        .read_to_end(&mut out)
        .is_ok()
    {
        out
    } else {
        body.to_vec()
    }
}

/// Inflate a `deflate` body.
///
/// RFC 1950 mandates the zlib wrapper, but several real-world servers send raw
/// deflate streams, so both are attempted before giving up.
fn inflate(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    if flate2::read::ZlibDecoder::new(body)
        .read_to_end(&mut out)
        .is_ok()
    {
        return out;
    }
    let mut out = Vec::new();
    if flate2::read::DeflateDecoder::new(body)
        .read_to_end(&mut out)
        .is_ok()
    {
        return out;
    }
    body.to_vec()
}

fn brotli_decompress(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut reader = brotli::Decompressor::new(body, 4096);
    if reader.read_to_end(&mut out).is_ok() {
        out
    } else {
        body.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_identity_when_absent() {
        assert_eq!(TransferEncoding::parse(None), TransferEncoding::Identity);
        assert_eq!(
            TransferEncoding::parse(Some("identity")),
            TransferEncoding::Identity
        );
    }

    #[test]
    fn parse_known_encodings() {
        assert_eq!(TransferEncoding::parse(Some("gzip")), TransferEncoding::Gzip);
        assert_eq!(
            TransferEncoding::parse(Some("BR")),
            TransferEncoding::Brotli
        );
        assert_eq!(
            TransferEncoding::parse(Some(" deflate ")),
            TransferEncoding::Deflate
        );
        assert_eq!(
            TransferEncoding::parse(Some("x-gzip")),
            TransferEncoding::Gzip
        );
    }

    #[test]
    fn parse_list_uses_innermost_encoding() {
        // Applied gzip then br: br must be undone first.
        assert_eq!(
            TransferEncoding::parse(Some("gzip, br")),
            TransferEncoding::Brotli
        );
        assert_eq!(
            TransferEncoding::parse(Some("br, gzip")),
            TransferEncoding::Gzip
        );
    }

    #[test]
    fn parse_unknown_encoding_is_unsupported() {
        assert_eq!(
            TransferEncoding::parse(Some("zstd")),
            TransferEncoding::Unsupported
        );
        assert!(TransferEncoding::parse(Some("zstd")).is_encoded());
        assert!(!TransferEncoding::Identity.is_encoded());
    }

    #[test]
    fn gzip_roundtrip() {
        let original = b"<html><body>kingston peptides</body></html>".repeat(40);
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(&original).expect("compress");
        let compressed = enc.finish().expect("finish");

        let decoded = decode(&compressed, TransferEncoding::Gzip);
        assert_eq!(decoded, original);
    }

    #[test]
    fn deflate_roundtrip() {
        let original = b"deflate roundtrip payload".repeat(50);
        let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(&original).expect("compress");
        let compressed = enc.finish().expect("finish");

        assert_eq!(decode(&compressed, TransferEncoding::Deflate), original);
    }

    #[test]
    fn brotli_roundtrip() {
        let original = b"brotli roundtrip payload".repeat(50);
        let mut compressed = Vec::new();
        {
            let mut enc = brotli::CompressorWriter::new(&mut compressed, 4096, 5, 22);
            enc.write_all(&original).expect("compress");
        }
        assert_eq!(decode(&compressed, TransferEncoding::Brotli), original);
    }

    #[test]
    fn identity_passes_through_untouched() {
        let body = b"plain".to_vec();
        assert_eq!(decode(&body, TransferEncoding::Identity), body);
    }

    #[test]
    fn corrupt_body_degrades_to_raw_bytes() {
        // Must not panic or return empty; the analyzer sees what was sent.
        let garbage = vec![0xff, 0x00, 0x13, 0x37];
        assert_eq!(decode(&garbage, TransferEncoding::Gzip), garbage);
        assert_eq!(decode(&garbage, TransferEncoding::Brotli), garbage);
    }

    #[test]
    fn empty_body_is_fine() {
        assert!(decode(&[], TransferEncoding::Gzip).is_empty());
        assert!(decode(&[], TransferEncoding::Brotli).is_empty());
    }
}
