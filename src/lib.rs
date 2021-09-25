mod error;

pub use crate::error::{Error, Result};
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use std::borrow::Cow;
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
    pub prefix: Option<String>,
    pub name: String,
    pub attributes: HashMap<String, String>, // q:attr="val" => {"q:attr": "val"}
    pub namespaces: HashMap<String, String>, // local namespace newly defined in attributes
    parent: Option<ElementId>,
    children: Vec<Node>,
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
    fn ne(&self, other: &Self) -> bool {
        self.id != other.id
    }
}

impl Element {
    pub fn has_parent(&self) -> bool {
        self.parent != None
    }

    pub fn get_parent(&self) -> Option<ElementId> {
        self.parent
    }

    pub fn get_children(&self) -> &Vec<Node> {
        &self.children
    }

    pub fn children_element(&self) -> Vec<ElementId> {
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

    pub fn remove_child_at(&mut self, index: usize) -> Result<Node> {
        let node = self.children.get(index).ok_or(Error::NotFound)?;
        if let Node::Element(_) = node {
            return Err(Error::IsAnElement);
        }
        Ok(self.children.remove(index))
    }

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
}

impl ReadOptions {
    pub fn default() -> ReadOptions {
        ReadOptions { standalone: false }
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
        doc.new_element(None, None, String::new(), HashMap::new(), HashMap::new());
        doc
    }

    fn new_element(
        &mut self,
        parent: Option<ElementId>,
        prefix: Option<String>,
        name: String,
        attributes: HashMap<String, String>,
        namespaces: HashMap<String, String>,
    ) -> ElementId {
        let elem = Element {
            id: self.counter,
            prefix,
            name,
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
        let elem = self.get_mut_element(id)?;
        elem.parent = Some(id); // All elementid references in Document are valid.

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
                    .expect("Element not found in children"),
            );
        }

        let new_parent_elem = self.get_mut_element(new_parentid)?;
        new_parent_elem.children.push(Node::Element(id));
        Ok(())
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
        if self.store.get(0).unwrap().children.len() > 0 {
            return Err(Error::NotEmpty);
        }
        let reader = Reader::from_str(str);
        self.read(reader)?;
        Ok(())
    }

    pub fn read_reader<R: BufRead>(&mut self, reader: R) -> Result<()> {
        if self.store.get(0).unwrap().children.len() > 0 {
            return Err(Error::NotEmpty);
        }
        let reader = Reader::from_reader(reader);
        self.read(reader)?;
        Ok(())
    }

    fn read<B: BufRead>(&mut self, mut reader: Reader<B>) -> Result<()> {
        reader.expand_empty_elements(true);
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
                    let raw_name = reader.decode(ev.name());
                    let splitted: Vec<&str> = raw_name.splitn(2, ":").collect();
                    let (prefix, name) = if splitted.len() > 1 {
                        let prefix = splitted[0].to_string();
                        let name = splitted[1].to_string();
                        (Some(prefix), name)
                    } else {
                        (None, splitted[0].to_string())
                    };
                    let mut namespaces = HashMap::new();
                    let attributes = ev
                        .attributes()
                        .map(|o| {
                            let o = o?;
                            let key = reader.decode(o.key).to_string();
                            let value = o.unescape_and_decode_value(&reader)?;
                            Ok((key, value))
                        })
                        .filter(|o| match *o {
                            Ok((ref key, ref value)) if key == "xmlns" => {
                                namespaces.insert(String::new(), value.clone());
                                false
                            }
                            Ok((ref key, ref value)) if key.starts_with("xmlns:") => {
                                namespaces.insert(key[6..].to_owned(), value.to_owned());
                                false
                            }
                            _ => true,
                        })
                        .collect::<Result<HashMap<String, String>>>()?;
                    let parent_id = element_stack.last().unwrap();
                    let element =
                        self.new_element(Some(*parent_id), prefix, name, attributes, namespaces);
                    let node = Node::Element(element);
                    self.get_mut_element(*parent_id)
                        .unwrap()
                        .children
                        .push(node);
                    element_stack.push(element);
                }
                Ok(Event::End(ref ev)) => {
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
                            let last_raw_name = match &last_element.prefix {
                                Some(prefix) => {
                                    Cow::Owned(format!("{}:{}", prefix, last_element.name))
                                }
                                None => Cow::Borrowed(&last_element.name),
                            };
                            if last_raw_name == raw_name {
                                last_element.children.extend(move_children);
                                break;
                            };
                            if last_element.children.len() > 0 {
                                last_element.children.extend(move_children);
                                move_children =
                                    std::mem::replace(&mut last_element.children, Vec::new());
                            }
                        }
                    } else {
                        element_stack.pop(); // quick-xml checks if tag names match for us
                    }
                }
                Ok(Event::Text(e)) => {
                    let node = Node::Text(e.unescape_and_decode(&reader)?);
                    let &id = element_stack.last().unwrap();
                    self.get_mut_element(id).unwrap().children.push(node);
                }
                Ok(Event::Eof) => return Ok(()),
                Err(e) => return Err(Error::from(e)),
                // TODO!
                _ => (), // Unimplemented
            }
        }
    }

    pub fn write(&self, writer: &mut impl Write) -> Result<()> {
        let root = self.get_root();
        let mut writer = Writer::new_with_indent(writer, ' ' as u8, 4);
        self.write_nodes(&mut writer, &root.children)?;
        writer.write_event(Event::Eof)?;
        Ok(())
    }

    fn write_nodes(&self, writer: &mut Writer<impl Write>, nodes: &Vec<Node>) -> Result<()> {
        for node in nodes {
            match node {
                Node::Element(eid) => self.write_element(writer, *eid)?,
                Node::Text(text) => {
                    writer.write_event(Event::Text(BytesText::from_escaped_str(text)))?
                }
            };
        }
        Ok(())
    }

    fn write_element(&self, writer: &mut Writer<impl Write>, id: ElementId) -> Result<()> {
        let elem = self.get_element(id).unwrap();
        let name = match &elem.prefix {
            Some(prefix) => Cow::Owned(format!("{}:{}", prefix, &elem.name)),
            None => Cow::Borrowed(&elem.name),
        };
        let name_bytes = name.as_bytes();
        let mut start = BytesStart::borrowed_name(name_bytes);
        for (key, val) in &elem.attributes {
            start.push_attribute((key.as_bytes(), val.as_bytes()));
        }
        for (prefix, val) in &elem.namespaces {
            let attr_name = if prefix.len() == 0 {
                "xmlns".to_string()
            } else {
                format!("{}:{}", prefix, val)
            };
            start.push_attribute((attr_name.as_bytes(), val.as_bytes()));
        }
        if elem.children.len() > 0 {
            writer.write_event(Event::Start(start))?;
            self.write_nodes(writer, &elem.children)?;
            writer.write_event(Event::End(BytesEnd::borrowed(name_bytes)))?;
        } else {
            writer.write_event(Event::Empty(start))?
        }
        Ok(())
    }
}
