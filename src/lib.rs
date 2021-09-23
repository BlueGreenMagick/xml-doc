mod error;

use crate::error::{Error, Result};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::collections::HashMap;

type ElementId = usize;
enum Node {
    Element(ElementId),
    Text(String),
}

struct Element {
    prefix: Option<String>,
    name: String,
    attributes: HashMap<String, String>, // q:attr="val" => {"q:attr": "val"}
    namespaces: HashMap<String, String>, // local namespace newly defined in attributes
    parent: ElementId,
    children: Vec<Node>,
}

struct Document {
    counter: ElementId, // == self.store.len()
    nodes: Vec<Node>,
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
        self.counter += 1;
        let elem = Element {
            prefix,
            name,
            attributes,
            namespaces,
            parent,
            children: vec![],
        };
        self.store.push(elem);
        self.counter
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
        let mut count = 0;
        let mut buf = Vec::new();
        let mut element_stack: Vec<ElementId> = Vec::new();

        loop {
            match reader.read_event(&mut buf) {
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
                    element_stack.pop();
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
