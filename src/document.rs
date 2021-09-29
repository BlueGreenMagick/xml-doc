use crate::element::{Element, ElementData};
use crate::error::{Error, Result};
use encoding_rs::{Decoder, Encoding, UTF_16BE, UTF_16LE, UTF_8};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use std::collections::HashMap;
use std::io::{BufRead, Read, Write};
use std::str::FromStr;

#[cfg(debug_assertions)]
macro_rules! debug {
    ($x:expr) => {
        println!("{:?}", $x)
    };
}

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

#[derive(Debug)]
pub enum Node {
    Element(Element),
    Text(String),
    Comment(String),
    CData(String),
    PI(String),
    DocType(String),
}

impl Node {
    pub fn as_element(&self) -> Option<Element> {
        match self {
            Self::Element(elem) => Some(*elem),
            _ => None,
        }
    }

    pub(crate) fn build_text_content<'a>(&self, document: &'a Document, buf: &'a mut String) {
        match self {
            Node::Element(elem) => elem.build_text_content(document, buf),
            Node::Text(text) => buf.push_str(text),
            Node::CData(text) => buf.push_str(text),
            Node::PI(text) => buf.push_str(text),
            _ => {}
        }
    }

    /// Returns content if node is `Text`, `CData`, or `PI`.
    /// If node is `Element`, return [Element::text_content()]
    ///
    /// Implementation of [Node.textContent](https://developer.mozilla.org/en-US/docs/Web/API/Node/textContent)
    pub fn text_content<'a>(&self, document: &'a Document) -> String {
        let mut buf = String::new();
        self.build_text_content(document, &mut buf);
        buf
    }
}

struct DecodeReader<R: Read> {
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
    fn new(reader: R, decoder: Option<Decoder>) -> DecodeReader<R> {
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

    fn set_decoder(&mut self, dec: Option<Decoder>) {
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

/// Represents a XML document.
///
/// Use [`parse_str()`], [`parse_reader()`], or [`from_str()`] to parse xml.
///
/// # Examples
/// ```
/// use easy_xml::Document;
/// use std::str::FromStr;
///
/// let mut doc = Document::from_str(r#"<?xml version="1.0" encoding="UTF-8"?>
/// <package>
///     <metadata>
///         <author>Lewis Carol</author>
///     </metadata>
/// </package>
/// "#).unwrap();
/// let author_elem = doc
///   .root_element()
///   .unwrap()
///   .find(&doc, "metadata")
///   .unwrap()
///   .find(&doc, "author")
///   .unwrap();
/// author_elem.set_text_content(&mut doc, "Lewis Carroll");
/// let xml = doc.write_str();
/// ```
///

#[derive(Debug)]
pub struct Document {
    pub read_opts: ReadOptions,
    pub(crate) counter: usize, // == self.store.len()
    pub(crate) store: Vec<ElementData>,
    container: Element,

    version: String,
    encoding: Option<String>,
    standalone: bool,
}

impl Document {
    /// Create a blank new xml document.
    pub fn new() -> Document {
        let (container, container_data) = Element::container();
        Document {
            read_opts: ReadOptions::default(),
            counter: 1, // because container is id 0
            store: vec![container_data],
            container,
            version: String::new(), // will be changed later
            encoding: None,
            standalone: false,
        }
    }

    pub fn container(&self) -> Element {
        self.container
    }

    pub fn is_empty(&self) -> bool {
        self.store.len() == 1
    }

    /// Get first element of document.
    pub fn root_element(&self) -> Option<Element> {
        self.container.child_elements(self).get(0).copied()
    }

    // Get root nodes of document.
    pub fn root_nodes(&self) -> &Vec<Node> {
        self.container.children(self)
    }
}

// Read and write
impl Document {
    /// Parses xml string. You can only call this from an empty document.
    ///
    /// # Errors
    ///
    /// Returns Errors from [`.read_reader()`].
    pub fn parse_str(&mut self, str: &str) -> Result<()> {
        if !self.is_empty() {
            return Err(Error::NotEmpty);
        }
        self.parse_reader(str.as_bytes())
    }

    /// Parses xml string from reader. You can only call this from an empty document.
    ///
    /// # Errors
    ///
    /// - [`Error::NotEmpty`]: You can only call this function on an empty document.
    /// - [`Error::CannotDecode`]: Could not decode XML.
    /// - [`Error::MalformedXML`]: Could not read XML.
    /// - [`Error::Io`]: IO Error
    pub fn parse_reader<R: Read>(&mut self, reader: R) -> Result<()> {
        if !self.is_empty() {
            return Err(Error::NotEmpty);
        }
        self.parse_start(reader)?;
        Ok(())
    }

    fn handle_decl(&mut self, ev: &BytesDecl) -> Result<()> {
        self.version = String::from_utf8(ev.version()?.to_vec())?;
        self.encoding = match ev.encoding() {
            Some(res) => Some(String::from_utf8(res?.to_vec())?),
            None => None,
        };
        self.standalone = match ev.standalone() {
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
        let full_name = String::from_utf8(ev.name().to_vec())?;
        let element = Element::new(self, full_name);
        let mut namespaces = HashMap::new();
        let attributes = element.mut_attributes(self);
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
        element.mut_namespace_declarations(self).extend(namespaces);
        let parent = *element_stack.last().unwrap();
        parent.push_child(self, Node::Element(element)).unwrap();
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

    fn parse_content<B: BufRead>(&mut self, mut reader: Reader<B>) -> Result<()> {
        let mut buf = Vec::with_capacity(200);
        let mut element_stack: Vec<Element> = vec![self.container()]; // container element in element_stack

        loop {
            let ev = reader.read_event(&mut buf);
            #[cfg(debug_assertions)]
            debug!(ev);
            match ev {
                Ok(Event::Start(ref ev)) => {
                    let element = self.handle_bytes_start(&element_stack, ev)?;
                    element_stack.push(element);
                }
                Ok(Event::End(_)) => {
                    let elem = element_stack.pop().unwrap(); // quick-xml checks if tag names match for us
                    if self.read_opts.empty_text_node {
                        // distinguish <tag></tag> and <tag />
                        if !elem.has_children(self) {
                            elem.push_child(self, Node::Text(String::new())).unwrap();
                        }
                    }
                }
                Ok(Event::Empty(ref ev)) => {
                    self.handle_bytes_start(&element_stack, ev)?;
                }
                Ok(Event::Text(ev)) => {
                    let content = String::from_utf8(ev.to_vec())?;
                    let node = Node::Text(content);
                    let elem = *element_stack.last().unwrap();
                    elem.push_child(self, node).unwrap();
                }
                Ok(Event::DocType(ev)) => {
                    let content = String::from_utf8(ev.to_vec())?;
                    let node = Node::DocType(content);
                    let elem = *element_stack.last().unwrap();
                    elem.push_child(self, node).unwrap();
                }
                // Comment, CData, and PI content is not escaped.
                Ok(Event::Comment(ev)) => {
                    let content = String::from_utf8(ev.unescaped()?.to_vec())?;
                    let node = Node::Comment(content);
                    let elem = *element_stack.last().unwrap();
                    elem.push_child(self, node).unwrap();
                }
                Ok(Event::CData(ev)) => {
                    let content = String::from_utf8(ev.unescaped()?.to_vec())?;
                    let node = Node::CData(content);
                    let elem = *element_stack.last().unwrap();
                    elem.push_child(self, node).unwrap();
                }
                Ok(Event::PI(ev)) => {
                    let content = String::from_utf8(ev.unescaped()?.to_vec())?;
                    let node = Node::PI(content);
                    let elem = *element_stack.last().unwrap();
                    elem.push_child(self, node).unwrap();
                }
                Ok(Event::Decl(ev)) => {
                    self.handle_decl(&ev)?;
                }
                Ok(Event::Eof) => return Ok(()),
                Err(e) => return Err(Error::from(e)),
            }
        }
    }

    /// Writes document as xml string.
    pub fn write_str(&self) -> Result<String> {
        let mut buf: Vec<u8> = Vec::with_capacity(200);
        self.write(&mut buf)?;
        Ok(String::from_utf8(buf)?)
    }

    /// Write document to writer. Will be written in UTF-8.
    pub fn write(&self, writer: &mut impl Write) -> Result<()> {
        let container = self.container();
        let mut writer = Writer::new_with_indent(writer, b' ', 4);
        self.write_decl(&mut writer)?;
        self.write_nodes(&mut writer, container.children(self))?;
        writer.write_event(Event::Eof)?;
        Ok(())
    }

    fn write_decl(&self, writer: &mut Writer<impl Write>) -> Result<()> {
        let standalone = match self.standalone {
            true => Some("yes".as_bytes()),
            false => None,
        };
        writer.write_event(Event::Decl(BytesDecl::new(
            self.version.as_bytes(),
            Some("UTF-8".as_bytes()),
            standalone,
        )))?;
        Ok(())
    }

    fn write_nodes(&self, writer: &mut Writer<impl Write>, nodes: &[Node]) -> Result<()> {
        for node in nodes {
            match node {
                Node::Element(eid) => self.write_element(writer, *eid)?,
                Node::Text(text) => {
                    writer.write_event(Event::Text(BytesText::from_plain_str(text)))?
                }
                Node::DocType(text) => {
                    writer.write_event(Event::DocType(BytesText::from_plain_str(text)))?
                }
                // Comment, CData, and PI content is not escaped.
                Node::Comment(text) => {
                    writer.write_event(Event::Comment(BytesText::from_escaped_str(text)))?
                }
                Node::CData(text) => {
                    writer.write_event(Event::CData(BytesText::from_escaped_str(text)))?
                }
                Node::PI(text) => {
                    writer.write_event(Event::PI(BytesText::from_escaped_str(text)))?
                }
            };
        }
        Ok(())
    }

    fn write_element(&self, writer: &mut Writer<impl Write>, element: Element) -> Result<()> {
        let name_bytes = element.full_name(self).as_bytes();
        let mut start = BytesStart::borrowed_name(name_bytes);
        for (key, val) in element.attributes(self) {
            start.push_attribute((key.as_bytes(), val.as_bytes()));
        }
        for (prefix, val) in element.namespace_declarations(self) {
            let attr_name = if prefix.is_empty() {
                "xmlns".to_string()
            } else {
                format!("xmlns:{}", prefix)
            };
            start.push_attribute((attr_name.as_bytes(), val.as_bytes()));
        }
        if element.has_children(self) {
            writer.write_event(Event::Start(start))?;
            self.write_nodes(writer, element.children(self))?;
            writer.write_event(Event::End(BytesEnd::borrowed(name_bytes)))?;
        } else {
            writer.write_event(Event::Empty(start))?;
        }
        Ok(())
    }
}

impl FromStr for Document {
    type Err = Error;

    fn from_str(s: &str) -> Result<Document> {
        let mut document = Document::new();
        document.parse_str(s)?;
        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_element() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <basic>
            Text
            <c />
        </basic>
        "#;
        let mut document = Document::from_str(xml).unwrap();
        let basic = document.container().children(&document)[0]
            .as_element()
            .unwrap();
        let p = Element::new(&mut document, "p");
        basic.push_child(&mut document, Node::Element(p)).unwrap();
        assert_eq!(p.parent(&document).unwrap(), basic);
        assert_eq!(
            p,
            basic
                .children(&document)
                .last()
                .unwrap()
                .as_element()
                .unwrap()
        )
    }
}
