mod error;

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

pub type ElementId = usize;

#[derive(Debug)]
pub enum Node {
    Element(ElementId),
    Text(String),
    Comment(String),
    CData(String),
    Decl {
        version: String,
        encoding: Option<String>,
        standalone: Option<String>,
    },
    PI(String),
    DocType(String),
}

impl Node {
    fn as_element(&self) -> Option<ElementId> {
        match self {
            Self::Element(id) => Some(*id),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Element {
    id: ElementId,
    pub raw_name: String,
    pub attributes: HashMap<String, String>, // q:attr="val" => {"q:attr": "val"}
    pub namespaces: HashMap<String, String>, // local namespace newly defined in attributes
    parent: Option<ElementId>,
    children: Vec<Node>,
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Element {
    pub fn get_prefix_name(&self) -> (&str, &str) {
        let splitted: Vec<&str> = self.raw_name.splitn(2, ':').collect();
        if splitted.len() == 1 {
            ("", splitted[0])
        } else {
            (splitted[0], splitted[1])
        }
    }

    pub fn get_name(&self) -> &str {
        self.get_prefix_name().1
    }

    pub fn get_prefix(&self) -> &str {
        self.get_prefix_name().0
    }

    pub fn has_parent(&self) -> bool {
        self.parent != None
    }

    pub fn get_parent(&self) -> Option<ElementId> {
        self.parent
    }

    pub fn get_children(&self) -> &Vec<Node> {
        &self.children
    }

    pub fn child_elements(&self) -> Vec<ElementId> {
        self.children
            .iter()
            .filter_map(|node| {
                if let Node::Element(elemid) = node {
                    Some(*elemid)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Insert a non element node to its children.
    ///
    /// # Errors
    /// - Error::IsAnElement if node is an element
    ///
    /// # Panics
    /// Panics if index > self.get_children().len()
    pub fn insert_child_node(&mut self, index: usize, node: Node) -> Result<()> {
        if let Node::Element(_) = node {
            return Err(Error::IsAnElement);
        }
        self.children.insert(index, node);
        Ok(())
    }

    /// Push a non element node to its children.
    pub fn push_child_node(&mut self, node: Node) -> Result<()> {
        if let Node::Element(_) = node {
            return Err(Error::IsAnElement);
        }
        self.children.push(node);
        Ok(())
    }

    pub fn remove_child_at(&mut self, index: usize) -> Result<Node> {
        let node = self.children.get(index).ok_or(Error::NotFound)?;
        if let Node::Element(_) = node {
            return Err(Error::IsAnElement);
        }
        Ok(self.children.remove(index))
    }

    // After calling this method, remove parent from child.
    fn remove_child_element(&mut self, id: ElementId) {
        let idx = self
            .children
            .iter()
            .position(|node| {
                if let Node::Element(e) = node {
                    if *e == id {
                        return true;
                    }
                }
                false
            })
            .unwrap();
        self.children.remove(idx);
    }
}

#[derive(Debug, Clone)]
pub struct ReadOptions {
    pub standalone: bool, // Whether to accept tags that doesn't have closing tags like <br>
    pub empty_text_node: bool, // <tag></tag> will have a Node::Text("") as its children, while <tag /> won't.
}

impl ReadOptions {
    pub fn default() -> ReadOptions {
        ReadOptions {
            standalone: false,
            empty_text_node: true,
        }
    }
}

#[derive(Debug)]
pub struct Document {
    counter: ElementId, // == self.store.len()
    store: Vec<Element>,
    pub read_opts: ReadOptions,
}

impl Document {
    pub fn new() -> Document {
        let mut doc = Document {
            counter: 0,
            store: vec![],
            read_opts: ReadOptions::default(),
        };
        // create root element
        doc.add_element(None, String::new(), HashMap::new(), HashMap::new());
        doc
    }

    pub fn create_element<S: Into<String>>(&mut self, name: S) -> &mut Element {
        let elemid = self.add_element(None, name.into(), HashMap::new(), HashMap::new());
        self.get_mut_element(elemid).unwrap()
    }

    fn add_element(
        &mut self,
        parent: Option<ElementId>,
        raw_name: String,
        attributes: HashMap<String, String>,
        namespaces: HashMap<String, String>,
    ) -> ElementId {
        let elem = Element {
            id: self.counter,
            raw_name,
            attributes,
            namespaces,
            parent,
            children: vec![],
        };
        self.store.push(elem);
        self.counter += 1;
        self.counter - 1
    }

    pub fn has_element(&self, id: ElementId) -> bool {
        self.counter > id
    }

    pub fn get_element(&self, id: ElementId) -> Result<&Element> {
        self.store.get(id).ok_or(Error::ElementNotExist(id))
    }

    pub fn get_mut_element(&mut self, id: ElementId) -> Result<&mut Element> {
        self.store.get_mut(id).ok_or(Error::ElementNotExist(id))
    }

    pub fn get_root(&self) -> &Element {
        self.store.get(0).expect("Root element is gone!")
    }

    pub fn get_mut_root(&mut self) -> &mut Element {
        self.store.get_mut(0).expect("Root element is gone!")
    }

    pub fn remove_from_parent(&mut self, id: ElementId) -> Result<()> {
        if id == 0 {
            return Err(Error::RootCannotMove);
        }
        let mut elem = self.get_mut_element(id)?;
        match elem.parent {
            None => return Ok(()),
            Some(parentid) => {
                elem.parent = None;
                let parent = self.get_mut_element(parentid).unwrap();
                parent.remove_child_element(id);
            }
        }
        Ok(())
    }

    pub fn set_parent(&mut self, id: ElementId, new_parentid: ElementId) -> Result<()> {
        if id == 0 {
            return Err(Error::RootCannotMove);
        }
        if !self.has_element(new_parentid) {
            return Err(Error::ElementNotExist(new_parentid));
        }
        let elem = self.get_element(id)?;

        if let Some(parentid) = elem.get_parent() {
            let parent_children = &mut self
                .get_mut_element(parentid)
                .expect("Document is inconsistant: Parent element doesn't exist.")
                .children;
            parent_children.remove(
                parent_children
                    .iter()
                    .filter_map(|node| node.as_element())
                    .position(|x| x == id)
                    .expect("Document is inconsistant: Element not found in children"),
            );
        }
        let elem = self.get_mut_element(id)?;
        elem.parent = Some(new_parentid);
        let new_parent_elem = self.get_mut_element(new_parentid)?;
        new_parent_elem.children.push(Node::Element(id));
        Ok(())
    }

    pub fn get_namespace(&self, id: ElementId, prefix: &str) -> Result<&str> {
        let mut id = id;
        while id != 0 {
            let elem = self.get_element(id)?;
            if let Some(value) = elem.namespaces.get(prefix) {
                return Ok(value);
            }
            id = elem.parent.ok_or(Error::NotFound)?;
        }
        Err(Error::NotFound)
    }
}

// Read and write
impl Document {
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

    pub fn read_str(&mut self, str: &str) -> Result<()> {
        if !self.get_root().children.is_empty() {
            return Err(Error::NotEmpty);
        }
        let reader = Reader::from_str(str);
        self.read(reader)?;
        Ok(())
    }

    pub fn read_reader<R: BufRead>(&mut self, reader: R) -> Result<()> {
        if !self.get_root().children.is_empty() {
            return Err(Error::NotEmpty);
        }
        let reader = Reader::from_reader(reader);
        self.read(reader)?;
        Ok(())
    }

    fn read_bytes_start<B: BufRead>(
        &mut self,
        reader: &Reader<B>,
        element_stack: &Vec<ElementId>,
        ev: &BytesStart,
    ) -> Result<ElementId> {
        let raw_name = reader.decode(ev.name()).to_string();
        let mut namespaces = HashMap::new();
        let mut attributes = HashMap::new();
        for attr in ev.attributes() {
            let attr = attr?;
            let key = reader.decode(attr.key).to_string();
            let value = attr.unescape_and_decode_value(reader)?;
            if key == "xmlns" {
                namespaces.insert(String::new(), value);
                continue;
            } else if let Some(prefix) = key.strip_prefix("xmlns:") {
                namespaces.insert(prefix.to_owned(), value);
                continue;
            }
            attributes.insert(key, value);
        }
        let parent_id = *element_stack.last().unwrap();
        let element = self.add_element(Some(parent_id), raw_name, attributes, namespaces);
        let node = Node::Element(element);
        self.get_mut_element(parent_id).unwrap().children.push(node);
        Ok(element)
    }

    fn read_bytes_end<B: BufRead>(
        &mut self,
        reader: &Reader<B>,
        element_stack: &mut Vec<ElementId>,
        ev: &BytesEnd,
    ) -> Result<()> {
        let opts_empty_text_node = self.read_opts.empty_text_node;
        if self.read_opts.standalone {
            let raw_name = reader.decode(ev.name());
            let mut move_children: Vec<Node> = vec![];
            loop {
                let last_eid = element_stack.pop().ok_or_else(|| {
                    Error::MalformedXML(format!(
                        "Closing tag mismatch: {}, pos: {}",
                        raw_name,
                        reader.buffer_position()
                    ))
                })?;
                let last_element = self.get_mut_element(last_eid).unwrap();
                let last_raw_name: &str = &last_element.raw_name;
                if last_raw_name == raw_name {
                    if !move_children.is_empty() {
                        last_element.children.extend(move_children);
                    } else if opts_empty_text_node && last_element.children.is_empty() {
                        last_element.children.push(Node::Text(String::new()));
                    }
                    break;
                };
                if !last_element.children.is_empty() {
                    last_element.children.extend(move_children);
                    move_children = std::mem::take(&mut last_element.children);
                }
            }
        } else {
            let elemid = element_stack.pop().unwrap(); // quick-xml checks if tag names match for us
            if opts_empty_text_node {
                let elem = self.get_mut_element(elemid).unwrap();
                // distinguish <tag></tag> and <tag />
                if elem.children.is_empty() {
                    elem.children.push(Node::Text(String::new()));
                }
            }
        }
        Ok(())
    }

    fn read<B: BufRead>(&mut self, mut reader: Reader<B>) -> Result<()> {
        reader.check_end_names(!self.read_opts.standalone);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut element_stack: Vec<ElementId> = vec![0]; // root element in element_stack

        loop {
            let ev = reader.read_event(&mut buf);
            #[cfg(debug_assertions)]
            debug!(ev);
            match ev {
                Ok(Event::Start(ref ev)) => {
                    let element = self.read_bytes_start(&reader, &element_stack, ev)?;
                    element_stack.push(element);
                }
                Ok(Event::End(ref ev)) => {
                    self.read_bytes_end(&reader, &mut element_stack, ev)?;
                }
                Ok(Event::Empty(ref ev)) => {
                    self.read_bytes_start(&reader, &element_stack, ev)?;
                }
                Ok(Event::Text(ev)) => {
                    let node = Node::Text(ev.unescape_and_decode(&reader)?);
                    let id = *element_stack.last().unwrap();
                    self.get_mut_element(id).unwrap().children.push(node);
                }
                Ok(Event::Comment(ev)) => {
                    let node = Node::Comment(ev.unescape_and_decode(&reader)?);
                    let id = *element_stack.last().unwrap();
                    self.get_mut_element(id).unwrap().children.push(node);
                }
                Ok(Event::CData(ev)) => {
                    let node = Node::CData(ev.unescape_and_decode(&reader)?);
                    let id = *element_stack.last().unwrap();
                    self.get_mut_element(id).unwrap().children.push(node);
                }
                Ok(Event::PI(ev)) => {
                    let node = Node::PI(ev.unescape_and_decode(&reader)?);
                    let id = *element_stack.last().unwrap();
                    self.get_mut_element(id).unwrap().children.push(node);
                }
                Ok(Event::DocType(ev)) => {
                    let node = Node::DocType(ev.unescape_and_decode(&reader)?);
                    let id = *element_stack.last().unwrap();
                    self.get_mut_element(id).unwrap().children.push(node);
                }
                Ok(Event::Decl(ev)) => {
                    let version = String::from_utf8_lossy(&ev.version()?).into_owned();
                    let encoding = match ev.encoding() {
                        Some(res) => Some(String::from_utf8_lossy(&res?).into_owned()),
                        None => None,
                    };
                    let standalone = match ev.standalone() {
                        Some(res) => Some(String::from_utf8_lossy(&res?).into_owned()),
                        None => None,
                    };
                    let node = Node::Decl {
                        version,
                        encoding,
                        standalone,
                    };
                    let id = *element_stack.last().unwrap();
                    self.get_mut_element(id).unwrap().children.push(node);
                }
                Ok(Event::Eof) => return Ok(()),
                Err(e) => return Err(Error::from(e)),
            }
        }
    }

    pub fn write(&self, writer: &mut impl Write) -> Result<()> {
        let root = self.get_root();
        let mut writer = Writer::new_with_indent(writer, b' ', 4);
        self.write_nodes(&mut writer, &root.children)?;
        writer.write_event(Event::Eof)?;
        Ok(())
    }

    fn write_nodes(&self, writer: &mut Writer<impl Write>, nodes: &[Node]) -> Result<()> {
        for node in nodes {
            match node {
                Node::Element(eid) => self.write_element(writer, *eid)?,
                Node::Text(text) => {
                    writer.write_event(Event::Text(BytesText::from_escaped_str(text)))?
                }
                Node::CData(text) => {
                    writer.write_event(Event::CData(BytesText::from_escaped_str(text)))?
                }
                Node::Comment(text) => {
                    writer.write_event(Event::Comment(BytesText::from_escaped_str(text)))?
                }
                Node::DocType(text) => {
                    writer.write_event(Event::DocType(BytesText::from_escaped_str(text)))?
                }
                Node::PI(text) => {
                    writer.write_event(Event::PI(BytesText::from_escaped_str(text)))?
                }
                Node::Decl {
                    version,
                    encoding,
                    standalone,
                } => writer.write_event(Event::Decl(BytesDecl::new(
                    version.as_bytes(),
                    encoding.as_ref().map(|s| s.as_bytes()),
                    standalone.as_ref().map(|s| s.as_bytes()),
                )))?,
            };
        }
        Ok(())
    }

    fn write_element(&self, writer: &mut Writer<impl Write>, id: ElementId) -> Result<()> {
        let elem = self.get_element(id).unwrap();
        let name_bytes = elem.raw_name.as_bytes();
        let mut start = BytesStart::borrowed_name(name_bytes);
        for (key, val) in &elem.attributes {
            start.push_attribute((key.as_bytes(), val.as_bytes()));
        }
        for (prefix, val) in &elem.namespaces {
            let attr_name = if prefix.is_empty() {
                "xmlns".to_string()
            } else {
                format!("{}:{}", prefix, val)
            };
            start.push_attribute((attr_name.as_bytes(), val.as_bytes()));
        }
        if elem.children.is_empty() {
            writer.write_event(Event::Empty(start))?;
        } else {
            writer.write_event(Event::Start(start))?;
            self.write_nodes(writer, &elem.children)?;
            writer.write_event(Event::End(BytesEnd::borrowed(name_bytes)))?;
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
        let basic = document.get_root().get_children()[0].as_element().unwrap();
        let p = document.create_element("p").id;
        document.set_parent(p, basic).unwrap();
        assert_eq!(document.get_element(p).unwrap().parent.unwrap(), basic);
        assert_eq!(
            p,
            document
                .get_element(basic)
                .unwrap()
                .children
                .last()
                .unwrap()
                .as_element()
                .unwrap()
        )
    }

    #[test]
    fn test_namespace() {
        let xml = r#"
        <root xmlns="ns", xmlns:p="pns">
            <p:foo xmlns="inner">
                Hello
            </p:foo>
            <p:bar xmlns:p="in2">
                <c />
                World!
            </p:bar>
        </root>"#;
        let document = Document::from_str(xml).unwrap();
        let root = document.get_root().get_children()[0].as_element().unwrap();
        let child_elements = document.get_element(root).unwrap().child_elements();
        let foo = *child_elements.get(0).unwrap();
        let bar = *child_elements.get(1).unwrap();
        let bar_elem = document.get_element(bar).unwrap();
        let c = bar_elem.child_elements()[0];
        let c_elem = document.get_element(c).unwrap();
        assert_eq!(c_elem.get_prefix_name(), ("", "c"));
        assert_eq!(bar_elem.raw_name, "p:bar");
        assert_eq!(bar_elem.get_prefix(), "p");
        assert_eq!(bar_elem.get_name(), "bar");
        assert_eq!(document.get_namespace(c, "").unwrap(), "ns");
        assert_eq!(document.get_namespace(c, "p").unwrap(), "in2");
        assert!(document.get_namespace(c, "random").is_err());
        assert_eq!(document.get_namespace(bar, "p").unwrap(), "in2");
        assert_eq!(document.get_namespace(foo, "").unwrap(), "inner");
        assert_eq!(document.get_namespace(foo, "p").unwrap(), "pns");
        assert_eq!(document.get_namespace(root, "").unwrap(), "ns");
    }
}
