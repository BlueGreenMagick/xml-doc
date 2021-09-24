mod error;

use crate::error::{Error, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::BufRead;

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
    pub children: Vec<Node>,
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
    pub fn get_parent(&self) -> Option<ElementId> {
        self.parent
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
}

#[derive(Debug)]
pub struct Document {
    counter: ElementId, // == self.store.len()
    store: Vec<Element>,
}

impl Document {
    fn empty() -> Document {
        let mut doc = Document {
            counter: 0,
            store: vec![],
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

    pub fn from_str(str: &str) -> Result<Document> {
        let mut document = Document::empty();
        let reader = Reader::from_str(str);
        document.build(reader)?;
        Ok(document)
    }

    pub fn from_reader<R: BufRead>(reader: R) -> Result<Document> {
        let mut document = Document::empty();
        let reader = Reader::from_reader(reader);
        document.build(reader)?;
        Ok(document)
    }

    fn build<B: BufRead>(&mut self, mut reader: Reader<B>) -> Result<()> {
        reader.expand_empty_elements(true);
        reader.check_end_names(false);
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
                    let parent = element_stack.last().copied();
                    let element = self.new_element(parent, prefix, name, attributes, namespaces);
                    let node = Node::Element(element);
                    //TODO: make sure unwrap isn't called.
                    let id = parent.unwrap();
                    self.get_mut_element(id).unwrap().children.push(node);
                    element_stack.push(element);
                }
                Ok(Event::End(ref ev)) => {
                    let raw_name = reader.decode(ev.name());
                    let mut move_children: Vec<Vec<Node>> = vec![];
                    loop {
                        let last_eid = element_stack.pop().ok_or(Error::MalformedXML(format!(
                            "Closing tag without corresponding opening tag: {}, pos: {}",
                            raw_name,
                            reader.buffer_position()
                        )))?;
                        let last_element = self.get_mut_element(last_eid).unwrap();
                        let last_raw_name = match &last_element.prefix {
                            Some(prefix) => Cow::Owned(format!("{}:{}", prefix, last_element.name)),
                            None => Cow::Borrowed(&last_element.name),
                        };
                        if *last_raw_name == raw_name {
                            while let Some(nodes) = move_children.pop() {
                                last_element.children.extend(nodes);
                            }
                            break;
                        };
                        if last_element.children.len() > 0 {
                            move_children
                                .push(std::mem::replace(&mut last_element.children, Vec::new()))
                        }
                    }
                }
                Ok(Event::Text(e)) => {
                    let node = Node::Text(e.unescape_and_decode(&reader)?);
                    let &id = element_stack.last().unwrap();
                    self.get_mut_element(id).unwrap().children.push(node);
                }
                Ok(Event::Eof) => return Ok(()),
                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
                // TODO!
                _ => (), // Unimplemented
            }
        }
    }
}
