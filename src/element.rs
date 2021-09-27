use super::error::{Error, Result};
use super::{Document, ElementData, Node};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Element {
    id: usize,
}

impl Element {
    pub fn new<S: Into<String>>(document: &mut Document, name: S) -> Element {
        Self::with_data(document, name.into(), HashMap::new(), HashMap::new())
    }

    pub(crate) fn with_data(
        document: &mut Document,
        raw_name: String,
        attributes: HashMap<String, String>,
        namespace_decls: HashMap<String, String>,
    ) -> Element {
        let elem = Element {
            id: document.counter,
        };
        let elem_data = ElementData {
            id: elem,
            raw_name,
            attributes,
            namespace_decls,
            parent: None,
            children: vec![],
        };
        document.store.push(elem_data);
        document.counter += 1;
        elem
    }

    pub fn is_root(&self) -> bool {
        self.id == 0
    }
}

impl Element {
    fn data<'a>(&self, document: &'a Document) -> &'a ElementData {
        document.store.get(self.id).unwrap()
    }

    fn mut_data<'a>(&self, document: &'a mut Document) -> &'a mut ElementData {
        document.store.get_mut(self.id).unwrap()
    }

    pub fn raw_name<'a>(&self, document: &'a Document) -> &'a str {
        &self.data(document).raw_name
    }

    pub fn prefix_name<'a>(&self, document: &'a Document) -> (&'a str, &'a str) {
        let data = self.data(document);
        match data.raw_name.split_once(":") {
            Some((prefix, name)) => (prefix, name),
            None => ("", &data.raw_name),
        }
    }

    pub fn prefix<'a>(&self, document: &'a Document) -> &'a str {
        self.prefix_name(document).0
    }

    pub fn name<'a>(&self, document: &'a Document) -> &'a str {
        self.prefix_name(document).1
    }

    pub fn attributes<'a>(&self, document: &'a Document) -> &'a HashMap<String, String> {
        &self.data(document).attributes
    }

    pub fn mut_attributes<'a>(
        &self,
        document: &'a mut Document,
    ) -> &'a mut HashMap<String, String> {
        &mut self.mut_data(document).attributes
    }

    pub fn namespace<'a>(&self, document: &'a Document) -> Option<&'a str> {
        self.namespace_for_prefix(document, self.prefix(document))
    }

    pub fn namespace_declarations<'a>(
        &self,
        document: &'a Document,
    ) -> &'a HashMap<String, String> {
        &self.data(document).namespace_decls
    }

    pub fn mut_namespace_declarations<'a>(
        &self,
        document: &'a mut Document,
    ) -> &'a mut HashMap<String, String> {
        &mut self.mut_data(document).namespace_decls
    }

    // TODO: check out https://www.w3.org/TR/xml-names/#ns-decl
    // about xmlns and xml prefix
    pub fn namespace_for_prefix<'a>(
        &self,
        document: &'a Document,
        prefix: &str,
    ) -> Option<&'a str> {
        let mut elem = *self;
        while !elem.is_root() {
            let data = elem.data(document);
            if let Some(value) = data.namespace_decls.get(prefix) {
                return Some(value);
            }
            elem = elem.parent(document)?;
        }
        None
    }

    pub fn parent(&self, document: &Document) -> Option<Element> {
        self.data(document).parent
    }

    pub fn has_parent(&self, document: &Document) -> bool {
        self.parent(document).is_some()
    }

    pub fn children<'a>(&self, document: &'a Document) -> &'a Vec<Node> {
        &self.data(document).children
    }

    pub fn has_children(&self, document: &Document) -> bool {
        !self.children(document).is_empty()
    }

    pub fn child_elements(&self, document: &Document) -> Vec<Element> {
        self.children(document)
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

    pub fn push_child(&self, document: &mut Document, node: Node) -> Result<()> {
        if let Node::Element(elem) = node {
            if elem.is_root() {
                return Err(Error::RootCannotMove);
            }
            let data = elem.mut_data(document);
            if data.parent.is_some() {
                return Err(Error::HasAParent);
            }
            data.parent = Some(*self);
        }
        self.mut_data(document).children.push(node);
        Ok(())
    }

    // if node is an element, the element must not have a parent.
    pub fn insert_child(&self, document: &mut Document, index: usize, node: Node) -> Result<()> {
        if let Node::Element(elem) = node {
            if elem.is_root() {
                return Err(Error::RootCannotMove);
            }
            let data = elem.mut_data(document);
            if data.parent.is_some() {
                return Err(Error::HasAParent);
            }
            data.parent = Some(*self);
        }
        self.mut_data(document).children.insert(index, node);
        Ok(())
    }

    pub fn remove_child(&self, document: &mut Document, index: usize) -> Node {
        self.mut_data(document).children.remove(index)
    }

    pub fn remove_child_elem(&self, document: &mut Document, element: Element) -> Result<()> {
        let children = &mut self.mut_data(document).children;
        let pos = children
            .iter()
            .filter_map(|n| {
                if let Node::Element(elem) = &n {
                    Some(*elem)
                } else {
                    None
                }
            })
            .position(|e| e == element)
            .ok_or(Error::NotFound)?;
        children.remove(pos);
        element.mut_data(document).parent = None;
        Ok(())
    }

    pub fn detatch_from_parent(&self, document: &mut Document) -> Result<()> {
        if self.is_root() {
            return Err(Error::RootCannotMove);
        }
        let parent = self.data(document).parent;
        if let Some(parent) = parent {
            parent.remove_child_elem(document, *self)
        } else {
            Ok(())
        }
    }
}
