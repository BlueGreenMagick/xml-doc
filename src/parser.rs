use crate::document::{Document, Node};
use crate::element::Element;
use crate::error::{Error, Result};
use encoding_rs::Decoder;
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE, UTF_8};
use quick_xml::events::{BytesDecl, BytesStart, Event};
use quick_xml::Reader;
use std::collections::HashMap;
use std::io::{BufRead, Read};

#[cfg(debug_assertions)]
macro_rules! debug {
    ($x:expr) => {
        println!("{:?}", $x)
    };
}

pub(crate) struct DecodeReader<R: Read> {
    decoder: Option<Decoder>,
    inner: R,
    undecoded: [u8; 4096],
    undecoded_pos: usize,
    undecoded_cap: usize,
    remaining: [u8; 32], // Is there an encoding with > 32 bytes for a char?
    decoded: [u8; 12288],
    decoded_pos: usize,
    decoded_cap: usize,
    done: bool,
}

impl<R: Read> DecodeReader<R> {
    // If Decoder is not set, don't decode.
    pub(crate) fn new(reader: R, decoder: Option<Decoder>) -> DecodeReader<R> {
        DecodeReader {
            decoder,
            inner: reader,
            undecoded: [0; 4096],
            undecoded_pos: 0,
            undecoded_cap: 0,
            remaining: [0; 32],
            decoded: [0; 12288],
            decoded_pos: 0,
            decoded_cap: 0,
            done: false,
        }
    }

    pub(crate) fn set_decoder(&mut self, dec: Option<Decoder>) {
        self.decoder = dec;
        self.done = false;
    }

    // Call this only when decoder is Some
    fn fill_buf_decode(&mut self) -> std::io::Result<&[u8]> {
        if self.decoded_pos >= self.decoded_cap {
            debug_assert!(self.decoded_pos == self.decoded_cap);
            if self.done {
                return Ok(&[]);
            }
            let remaining = self.undecoded_cap - self.undecoded_pos;
            if remaining <= 32 {
                // Move remaining undecoded bytes at the end to start
                self.remaining[..remaining]
                    .copy_from_slice(&self.undecoded[self.undecoded_pos..self.undecoded_cap]);
                self.undecoded[..remaining].copy_from_slice(&self.remaining[..remaining]);
                // Fill undecoded buffer
                let read = self.inner.read(&mut self.undecoded[remaining..])?;
                self.done = read == 0;
                self.undecoded_pos = 0;
                self.undecoded_cap = remaining + read;
            }

            // Fill decoded buffer
            let (_res, read, written, _replaced) = self.decoder.as_mut().unwrap().decode_to_utf8(
                &self.undecoded[self.undecoded_pos..self.undecoded_cap],
                &mut self.decoded,
                self.done,
            );
            self.undecoded_pos += read;
            self.decoded_cap = written;
            self.decoded_pos = 0;
        }
        Ok(&self.decoded[self.decoded_pos..self.decoded_cap])
    }

    fn fill_buf_without_decode(&mut self) -> std::io::Result<&[u8]> {
        if self.undecoded_pos >= self.undecoded_cap {
            debug_assert!(self.undecoded_pos == self.undecoded_cap);
            self.undecoded_cap = self.inner.read(&mut self.undecoded)?;
            self.undecoded_pos = 0;
        }
        Ok(&self.undecoded[self.undecoded_pos..self.undecoded_cap])
    }
}

impl<R: Read> Read for DecodeReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        (&self.decoded[..]).read(buf)
    }
}

impl<R: Read> BufRead for DecodeReader<R> {
    // Decoder may change from None to Some.
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        match &self.decoder {
            Some(_) => self.fill_buf_decode(),
            None => self.fill_buf_without_decode(),
        }
    }
    fn consume(&mut self, amt: usize) {
        match &self.decoder {
            Some(_) => {
                self.decoded_pos = std::cmp::min(self.decoded_pos + amt, self.decoded_cap);
            }
            None => {
                self.undecoded_pos = std::cmp::min(self.undecoded_pos + amt, self.undecoded_cap);
            }
        }
    }
}

/// Options when parsing xml.
///
/// `empty_text_node`: <tag></tag> will have a Node::Text("") as its children, while <tag /> won't.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadOptions {
    pub empty_text_node: bool,
}

impl ReadOptions {
    pub fn default() -> ReadOptions {
        ReadOptions {
            empty_text_node: true,
        }
    }
}

pub(crate) struct DocumentParser {
    document: Document,
    read_opts: ReadOptions,
    encoding: Option<String>,
}

impl DocumentParser {
    pub(crate) fn new(opts: ReadOptions) -> DocumentParser {
        DocumentParser {
            document: Document::new(),
            read_opts: opts,
            encoding: None,
        }
    }

    pub(crate) fn parse_reader<R: Read>(reader: R, opts: ReadOptions) -> Result<Document> {
        let mut parser = DocumentParser::new(opts);
        parser.parse_start(reader)?;
        Ok(parser.document)
    }

    fn handle_decl(&mut self, ev: &BytesDecl) -> Result<()> {
        self.document.version = String::from_utf8(ev.version()?.to_vec())?;
        self.encoding = match ev.encoding() {
            Some(res) => Some(String::from_utf8(res?.to_vec())?),
            None => None,
        };
        self.document.standalone = match ev.standalone() {
            Some(res) => {
                let val = std::str::from_utf8(&*res?)?.to_lowercase();
                if val == "yes" {
                    true
                } else if val == "no" {
                    false
                } else {
                    return Err(Error::MalformedXML(
                        "Standalone Document Declaration has non boolean value".to_string(),
                    ));
                }
            }
            None => false,
        };
        Ok(())
    }

    fn handle_bytes_start(
        &mut self,
        element_stack: &Vec<Element>,
        ev: &BytesStart,
    ) -> Result<Element> {
        let mut_doc = &mut self.document;
        let full_name = String::from_utf8(ev.name().to_vec())?;
        let element = Element::new(mut_doc, full_name);
        let mut namespaces = HashMap::new();
        let attributes = element.mut_attributes(mut_doc);
        for attr in ev.attributes() {
            let attr = attr?;
            let key = String::from_utf8(attr.key.to_vec())?;
            let value = String::from_utf8(attr.unescaped_value()?.to_vec())?;
            if key == "xmlns" {
                namespaces.insert(String::new(), value);
                continue;
            } else if let Some(prefix) = key.strip_prefix("xmlns:") {
                namespaces.insert(prefix.to_owned(), value);
                continue;
            }
            attributes.insert(key, value);
        }
        element.mut_namespace_decls(mut_doc).extend(namespaces);
        let parent = *element_stack.last().unwrap();
        parent.push_child(mut_doc, Node::Element(element)).unwrap();
        Ok(element)
    }

    // Look at the document decl and figure out the document encoding
    fn parse_start<B: Read>(&mut self, reader: B) -> Result<()> {
        let mut bufreader = DecodeReader::new(reader, None);

        let bytes = bufreader.fill_buf()?;
        let init_encoding = match bytes {
            [0xfe, 0xff, ..] => {
                // UTF-16 BE BOM
                bufreader.consume(2);
                Some(UTF_16BE)
            }
            [0xff, 0xfe, ..] => {
                // UTF-16 LE BOM
                bufreader.consume(2);
                Some(UTF_16LE)
            }
            [0xef, 0xbb, 0xbf, ..] => {
                // UTF-8 BOM
                bufreader.consume(3);
                None
            }
            [0x00, 0x3c, 0x00, 0x3f, ..] => Some(UTF_16BE),
            [0x3c, 0x00, 0x3f, 0x00, ..] => Some(UTF_16LE),
            [0x3c, 0x3f, ..] => None,
            /*
            [0x00, 0x00, 0xfe, 0xff, ..] => return Err(Error::CannotDecode), // UTF-32 BE
            [0xff, 0xfe, 0x00, 0x00, ..] => return Err(Error::CannotDecode), // UTF-32 LE
            [0x00, 0x00, 0x00, 0x3c, ..] => return Err(Error::CannotDecode), // UTF-32 BE
            [0x3c, 0x00, 0x00, 0x00, ..] => return Err(Error::CannotDecode), // UTF-32 LE
             */
            _ => return Err(Error::CannotDecode), // TODO: allow having comments and text above Decl for Utf-8?
        };
        bufreader.set_decoder(init_encoding.map(|e| e.new_decoder_without_bom_handling()));
        let mut xmlreader = Reader::from_reader(bufreader);
        xmlreader.trim_text(true);
        let mut buf = Vec::with_capacity(150);
        let event = xmlreader.read_event(&mut buf)?;
        if let Event::Decl(ev) = event {
            self.handle_decl(&ev)?;
            if let Some(encoding_str) = &self.encoding {
                let encoding =
                    Encoding::for_label(encoding_str.as_bytes()).ok_or(Error::CannotDecode)?;
                let encoding = if encoding == UTF_8 {
                    None
                } else {
                    Some(encoding)
                };
                // Encoding::for_label("UTF-16") defaults to UTF-16 LE, even though it could be UTF-16 BE
                if encoding != init_encoding
                    && !(encoding == Some(UTF_16LE) && init_encoding == Some(UTF_16BE))
                {
                    let mut decode_reader = xmlreader.into_underlying_reader();
                    decode_reader
                        .set_decoder(encoding.map(|e| e.new_decoder_without_bom_handling()));
                    xmlreader = Reader::from_reader(decode_reader);
                    xmlreader.trim_text(true);
                }
            }
            self.parse_content(xmlreader)
        } else {
            Err(Error::MalformedXML(
                "Didn't find XML Declaration at the start of file".to_string(),
            ))
        }
    }

    // Returns if document parsing is finished.
    fn handle_event(&mut self, element_stack: &mut Vec<Element>, event: Event) -> Result<bool> {
        let mut_doc = &mut self.document;
        match event {
            Event::Start(ref ev) => {
                let element = self.handle_bytes_start(&element_stack, ev)?;
                element_stack.push(element);
                Ok(false)
            }
            Event::End(_) => {
                let elem = element_stack.pop().unwrap(); // quick-xml checks if tag names match for us
                if self.read_opts.empty_text_node {
                    // distinguish <tag></tag> and <tag />
                    if !elem.has_children(mut_doc) {
                        elem.push_child(mut_doc, Node::Text(String::new())).unwrap();
                    }
                }
                Ok(false)
            }
            Event::Empty(ref ev) => {
                self.handle_bytes_start(&element_stack, ev)?;
                Ok(false)
            }
            Event::Text(ev) => {
                let content = String::from_utf8(ev.to_vec())?;
                let node = Node::Text(content);
                let elem = *element_stack.last().unwrap();
                elem.push_child(mut_doc, node).unwrap();
                Ok(false)
            }
            Event::DocType(ev) => {
                let content = String::from_utf8(ev.to_vec())?;
                let node = Node::DocType(content);
                let elem = *element_stack.last().unwrap();
                elem.push_child(mut_doc, node).unwrap();
                Ok(false)
            }
            // Comment, CData, and PI content is not escaped.
            Event::Comment(ev) => {
                let content = String::from_utf8(ev.unescaped()?.to_vec())?;
                let node = Node::Comment(content);
                let elem = *element_stack.last().unwrap();
                elem.push_child(mut_doc, node).unwrap();
                Ok(false)
            }
            Event::CData(ev) => {
                let content = String::from_utf8(ev.unescaped()?.to_vec())?;
                let node = Node::CData(content);
                let elem = *element_stack.last().unwrap();
                elem.push_child(mut_doc, node).unwrap();
                Ok(false)
            }
            Event::PI(ev) => {
                let content = String::from_utf8(ev.unescaped()?.to_vec())?;
                let node = Node::PI(content);
                let elem = *element_stack.last().unwrap();
                elem.push_child(mut_doc, node).unwrap();
                Ok(false)
            }
            Event::Decl(ev) => {
                self.handle_decl(&ev)?;
                Ok(false)
            }
            Event::Eof => Ok(true),
        }
    }

    fn parse_content<B: BufRead>(&mut self, mut reader: Reader<B>) -> Result<()> {
        let mut buf = Vec::with_capacity(200); // reduce time increasing capacity at start.
        let mut element_stack: Vec<Element> = vec![self.document.container()]; // container element in element_stack

        loop {
            let ev = reader.read_event(&mut buf)?;
            #[cfg(debug_assertions)]
            debug!(ev);
            if self.handle_event(&mut element_stack, ev)? {
                return Ok(());
            }
        }
    }
}
