mod error;

use crate::error::{Error, Result};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::collections::HashMap;

pub type ElementId = usize;

#[cfg(debug_assertions)]
macro_rules! debug {
    ($x:expr) => {
        println!("{:?}", $x)
    };
}

#[cfg(not(debug_assertions))]
macro_rules! debug {
    ($x:expr) => {
        std::convert::identity($x)
    };
}

#[derive(Debug)]
pub enum Node {
    Element(ElementId),
    Text(String),
}

#[derive(Debug)]
pub struct Element {
    pub prefix: Option<String>,
    pub name: String,
    pub attributes: HashMap<String, String>, // q:attr="val" => {"q:attr": "val"}
    pub namespaces: HashMap<String, String>, // local namespace newly defined in attributes
    pub parent: ElementId,
    pub children: Vec<Node>,
}

#[derive(Debug)]
pub struct Document {
    counter: ElementId, // == self.store.len()
    pub nodes: Vec<Node>,
    store: Vec<Element>,
}

impl Document {
    fn empty() -> Document {
        Document {
            counter: 0,
            nodes: vec![],
            store: vec![],
        }
    }

    fn new_element(
        &mut self,
        parent: ElementId,
        prefix: Option<String>,
        name: String,
        attributes: HashMap<String, String>,
        namespaces: HashMap<String, String>,
    ) -> ElementId {
        let elem = Element {
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

    pub fn get_element(&self, id: ElementId) -> Option<&Element> {
        self.store.get(id)
    }

    pub fn get_mut_element(&mut self, id: ElementId) -> Option<&mut Element> {
        self.store.get_mut(id)
    }

    pub fn from_str(str: &str) -> Result<Document> {
        let mut document = Document::empty();
        let reader = Reader::from_str(str);
        document.build(reader)?;
        Ok(document)
    }

    fn build<B: std::io::BufRead>(&mut self, mut reader: Reader<B>) -> Result<()> {
        reader.expand_empty_elements(true);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut element_stack: Vec<ElementId> = Vec::new();

        loop {
            let ev = reader.read_event(&mut buf);
            debug!(ev);
            match ev {
                Ok(Event::Start(ref ev)) => {
                    let raw_name = reader.decode(ev.name());
                    let splitted: Vec<&str> = raw_name.splitn(2, ":").collect();
                    let (prefix, name) = if splitted.len() > 1 {
                        let prefix = splitted[0].to_string();
                        let name = splitted[0].to_string();
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
                    let parent = match element_stack.last() {
                        Some(&id) => id,
                        None => ElementId::MAX,
                    };
                    let element = self.new_element(parent, prefix, name, attributes, namespaces);
                    let node = Node::Element(element);
                    match element_stack.last() {
                        Some(&id) => self.get_mut_element(id).unwrap().children.push(node),
                        None => self.nodes.push(node),
                    };
                    element_stack.push(element);
                }
                Ok(Event::End(ref ev)) => {
                    element_stack.pop(); // TODO: check name of end element
                }
                Ok(Event::Text(e)) => {
                    let node = Node::Text(e.unescape_and_decode(&reader)?);
                    match element_stack.last() {
                        Some(&id) => self.get_mut_element(id).unwrap().children.push(node),
                        None => self.nodes.push(node),
                    }
                }
                Ok(Event::Eof) => return Ok(()),
                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
                // TODO!
                _ => (), // Unimplemented
            }
        }
    }
}
