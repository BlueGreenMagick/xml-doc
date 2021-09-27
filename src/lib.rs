mod element;
mod error;

pub use crate::element::{Element, ElementData};
pub use crate::error::{Error, Result};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use std::collections::HashMap;
use std::io::{BufRead, Write};

#[cfg(debug_assertions)]
macro_rules! debug {
    ($x:expr) => {
        println!("{:?}", $x)
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadOptions {
    pub empty_text_node: bool, // <tag></tag> will have a Node::Text("") as its children, while <tag /> won't.
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
}
#[derive(Debug)]
pub struct Document {
    pub read_opts: ReadOptions,
    counter: usize, // == self.store.len()
    store: Vec<ElementData>,
    root: Element,

    version: String,
    encoding: Option<String>,
    standalone: bool,
}

impl Document {
    pub fn new() -> Document {
        let (root, root_data) = Element::root();
        let doc = Document {
            read_opts: ReadOptions::default(),
            counter: 1, // because root is id 0
            store: vec![root_data],
            root,
            version: String::new(), // will be changed later
            encoding: None,
            standalone: false,
        };
        // create root element
        doc
    }

    pub fn root(&self) -> Element {
        self.root
    }

    pub fn is_empty(&self) -> bool {
        self.store.len() == 1
    }
}

// Read and write
impl Document {
    /// Create [`Document`] from xml string.
    pub fn from_str(str: &str) -> Result<Document> {
        let mut document = Document::new();
        document.read_str(str)?;
        Ok(document)
    }

    pub fn from_reader<R: BufRead>(reader: R) -> Result<Document> {
        let mut document = Document::new();
        document.read_reader(reader)?;
        Ok(document)
    }
    /// Parses xml string.
    ///
    /// # Errors
    ///
    /// - [`Error::NotEmpty`]: You can only call this function on an empty document.
    pub fn read_str(&mut self, str: &str) -> Result<()> {
        if !self.is_empty() {
            return Err(Error::NotEmpty);
        }
        let reader = Reader::from_str(str);
        self.read(reader)?;
        Ok(())
    }

    /// Parses xml string from reader.
    ///
    /// # Errors
    ///
    /// - [`Error::NotEmpty`]: You can only call this function on an empty document.
    pub fn read_reader<R: BufRead>(&mut self, reader: R) -> Result<()> {
        if !self.is_empty() {
            return Err(Error::NotEmpty);
        }
        let reader = Reader::from_reader(reader);
        self.read(reader)?;
        Ok(())
    }

    fn handle_decl(&mut self, ev: &BytesDecl) -> Result<()> {
        fn strip_quote<'a>(bytes: &'a [u8]) -> Result<&'a str> {
            let b = bytes[0];
            if b != b'"' && b == b'\'' {
                return Err(Error::MalformedXML(
                    "Attribute value without quotes".to_string(),
                ));
            }
            let inner = &bytes[1..bytes.len()];
            Ok(std::str::from_utf8(inner)?)
        }
        self.version = strip_quote(&ev.version()?)?.to_string();
        self.encoding = match ev.encoding() {
            Some(res) => Some(strip_quote(&res?)?.to_string()),
            None => None,
        };
        self.standalone = match ev.standalone() {
            Some(res) => {
                let val = strip_quote(&res?)?.to_lowercase();
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

    fn read<B: BufRead>(&mut self, mut reader: Reader<B>) -> Result<()> {
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut element_stack: Vec<Element> = vec![self.root()]; // root element in element_stack

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
        let mut buf: Vec<u8> = Vec::new();
        self.write(&mut buf)?;
        Ok(String::from_utf8(buf).unwrap())
    }

    pub fn write(&self, writer: &mut impl Write) -> Result<()> {
        let root = self.root();
        let mut writer = Writer::new_with_indent(writer, b' ', 4);
        self.write_decl(&mut writer)?;
        self.write_nodes(&mut writer, root.children(self))?;
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
            self.encoding.as_ref().map(|s| s.as_bytes()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_element() {
        let xml = r#"
        <basic>
            Text
            <c />
        </basic>
        "#;
        let mut document = Document::from_str(xml).unwrap();
        let basic = document.root().children(&document)[0].as_element().unwrap();
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
