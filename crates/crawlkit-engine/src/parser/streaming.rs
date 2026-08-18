//! Incremental HTML parsing over chunked streams.

use url::Url;

#[cfg(feature = "full")]
use tokio::sync::mpsc;

use super::links::LinkExtractor;
use super::{ExtractedLink, HtmlParser, MetaTags, ParseError, ParsedPage};

/// Events emitted by the streaming HTML parser.
///
/// Each event represents a stage in the incremental parsing pipeline.
/// Consumers can react to intermediate results (links, meta) as they become
/// available, without waiting for the full document.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::parser::ParserEvent;
///
/// let event = ParserEvent::Chunk(1024);
/// assert!(matches!(event, ParserEvent::Chunk(1024)));
/// ```
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum ParserEvent {
    /// A chunk of HTML has been received and appended to the buffer.
    Chunk(usize),
    /// Links extracted from the current partial document.
    Links(Vec<ExtractedLink>),
    /// Meta tags extracted from the current partial document.
    Meta(MetaTags),
    /// Parsing complete — full [`ParsedPage`] available.
    Done(Box<ParsedPage>),
    /// Error during parsing.
    Error(String),
}

impl HtmlParser {
    /// Parse an HTML document from a streaming source.
    ///
    /// Accepts a channel receiver that yields HTML chunks and returns a channel
    /// receiver that yields [`ParserEvent`]s as parsing progresses. Links and
    /// meta tags are extracted incrementally from the accumulated buffer, and
    /// a full [`ParsedPage`] is produced when the stream ends.
    ///
    /// This is useful for processing large HTML responses without buffering
    /// the entire body before parsing begins.
    ///
    /// # Arguments
    ///
    /// * `receiver` — Channel producing raw HTML byte chunks.
    /// * `base_url` — Base URL for resolving relative links.
    ///
    /// # Returns
    ///
    /// A channel receiver that yields parser events. The channel closes when
    /// parsing completes or an error occurs.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use crawlkit_engine::HtmlParser;
    /// use crawlkit_engine::parser::ParserEvent;
    /// use tokio::sync::mpsc;
    /// use url::Url;
    ///
    /// # async fn example() {
    /// let (tx, rx) = mpsc::channel::<Vec<u8>>(16);
    /// let base_url = Url::parse("https://example.com").unwrap();
    /// let mut events = HtmlParser::parse_stream(rx, base_url);
    ///
    /// // Feed chunks from an HTTP response
    /// tokio::spawn(async move {
    ///     let _ = tx.send(b"<html><head><title>T</title></head>".to_vec()).await;
    ///     let _ = tx.send(b"<body><a href=\"/link\">L</a></body></html>".to_vec()).await;
    ///     // tx dropped → stream ends
    /// });
    ///
    /// while let Some(event) = events.recv().await {
    ///     match event {
    ///         ParserEvent::Links(links) => println!("found {} links", links.len()),
    ///         ParserEvent::Done(page) => println!("title: {:?}", page.meta.title),
    ///         _ => {}
    ///     }
    /// }
    /// # }
    /// ```
    #[cfg(feature = "full")]
    pub fn parse_stream(
        mut receiver: mpsc::Receiver<Vec<u8>>,
        base_url: Url,
    ) -> mpsc::Receiver<ParserEvent> {
        let (tx, rx) = mpsc::channel(64);

        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut link_extractor = LinkExtractor::new(&base_url);

            while let Some(chunk) = receiver.recv().await {
                let chunk_str = String::from_utf8_lossy(&chunk);
                buffer.push_str(&chunk_str);

                let chunk_len = chunk.len();

                // Extract links synchronously, drop Html before .await
                let links = {
                    let doc = scraper::Html::parse_document(&buffer);
                    link_extractor.extract_links(&doc)
                };

                let _ = tx.send(ParserEvent::Chunk(chunk_len)).await;
                if !links.is_empty() {
                    let _ = tx.send(ParserEvent::Links(links)).await;
                }
            }

            match HtmlParser::parse(&buffer, &base_url) {
                Ok(page) => {
                    let _ = tx.send(ParserEvent::Meta(page.meta.clone())).await;
                    let _ = tx.send(ParserEvent::Done(Box::new(page))).await;
                }
                Err(e) => {
                    let _ = tx.send(ParserEvent::Error(e.to_string())).await;
                }
            }
        });

        rx
    }
}

/// Streaming HTML parser that processes content incrementally.
///
/// Buffers HTML chunks as they arrive and parses when a complete document
/// is detected. This is useful for processing HTML from streaming sources
/// like HTTP responses.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::parser::StreamingHtmlParser;
///
/// let mut parser = StreamingHtmlParser::new();
/// parser.feed("<!DOCTYPE html><html><head><title>Test</title></head>");
/// parser.feed("<body><h1>Hello</h1></body></html>");
///
/// assert!(parser.has_complete_document());
/// let page = parser.parse().unwrap();
/// assert_eq!(page.meta.title.as_deref(), Some("Test"));
/// ```
pub struct StreamingHtmlParser {
    buffer: String,
}

impl StreamingHtmlParser {
    /// Create a new streaming parser.
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Feed a chunk of HTML content into the parser.
    ///
    /// The chunk is appended to the internal buffer for later parsing.
    pub fn feed(&mut self, chunk: &str) {
        self.buffer.push_str(chunk);
    }

    /// Check if the accumulated content contains a complete HTML document.
    ///
    /// Returns `true` if the buffer contains `</html>` or `</body>` tags,
    /// indicating the document is likely complete.
    pub fn has_complete_document(&self) -> bool {
        self.buffer.contains("</html>") || self.buffer.contains("</body>")
    }

    /// Parse the accumulated HTML content.
    ///
    /// Delegates to [`HtmlParser::parse`] with a blank URL.
    /// Returns a [`ParsedPage`] containing all extracted SEO data.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if parsing fails (currently never happens).
    pub fn parse(&mut self) -> Result<ParsedPage, ParseError> {
        let url = url::Url::parse("about:blank")?;
        HtmlParser::parse(&self.buffer, &url)
    }

    /// Get the current buffer size in bytes.
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    /// Clear the internal buffer.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Get a reference to the buffered content.
    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    /// Consume the parser and return the buffered content.
    pub fn into_inner(self) -> String {
        self.buffer
    }
}

impl Default for StreamingHtmlParser {
    fn default() -> Self {
        Self::new()
    }
}
