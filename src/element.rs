use super::error::{Error, Result};
use super::{Document, Node};
use std::collections::HashMap;

/// Represents a XML document.
#[derive(Debug, PartialEq, Eq)]
pub struct ElementData {
    raw_name: String,
    attributes: HashMap<String, String>, // q:attr="val" => {"q:attr": "val"}
    namespace_decls: HashMap<String, String>, // local namespace newly defined in attributes
    parent: Option<Element>,
    children: Vec<Node>,
}

/// Represents an Xml Element.
///
/// This struct only contains a unique usize id and implements trait `Copy`.
/// So you do not need to bother with having a reference.
///
/// Because the actual data of the element is stored in [`Document`],
/// most methods takes `&Document` or `&mut Document` as its first argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Element {
    id: usize,
}

impl Element {
    /// Create a new empty element with name.
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

    pub(crate) fn root() -> (Element, ElementData) {
        let elem_data = ElementData {
            raw_name: String::new(),
            attributes: HashMap::new(),
            namespace_decls: HashMap::new(),
            parent: None,
            children: Vec::new(),
        };
        let elem = Element { id: 0 };
        return (elem, elem_data);
    }

    pub fn is_root(&self) -> bool {
        self.id == 0
    }

    pub fn seperate_prefix_name<'a>(raw_name: &'a str) -> (&'a str, &'a str) {
        match raw_name.split_once(":") {
            Some((prefix, name)) => (prefix, name),
            None => ("", &raw_name),
        }
    }
}

impl Element {
    fn data<'a>(&self, document: &'a Document) -> &'a ElementData {
        document.store.get(self.id).unwrap()
    }

    fn mut_data<'a>(&self, document: &'a mut Document) -> &'a mut ElementData {
        document.store.get_mut(self.id).unwrap()
    }

    /// Get raw name of element, including its namespace prefix.
    pub fn raw_name<'a>(&self, document: &'a Document) -> &'a str {
        &self.data(document).raw_name
    }

    /// Get prefix and name of element.
    ///
    /// `<prefix: name` -> `("prefix", "name")`
    pub fn prefix_name<'a>(&self, document: &'a Document) -> (&'a str, &'a str) {
        let data = self.data(document);
        Self::seperate_prefix_name(&data.raw_name)
    }

    /// Get namespace prefix of element, without name.
    ///
    /// `<prefix:name>` -> `"prefix"`.
    pub fn prefix<'a>(&self, document: &'a Document) -> &'a str {
        self.prefix_name(document).0
    }

    pub fn name<'a>(&self, document: &'a Document) -> &'a str {
        self.prefix_name(document).1
    }

    /// Get attributes of element.
    ///
    /// The attribute names may have namespace prefix. To strip the prefix and only its name, call [`Element::seperate_prefix_name`].
    /// ```ignore
    /// let attrs = element.attributes(&document);
    /// for attr in attrs {
    ///     let (prefix, name) = Element::seperate_prefix_name(attr);
    /// }
    /// ```
    pub fn attributes<'a>(&self, document: &'a Document) -> &'a HashMap<String, String> {
        &self.data(document).attributes
    }
    pub fn mut_attributes<'a>(
        &self,
        document: &'a mut Document,
    ) -> &'a mut HashMap<String, String> {
        &mut self.mut_data(document).attributes
    }

    /// Gets the namespace of this element.
    ///
    /// Shorthand for `self.namespace_for_prefix(document, self.prefix(document))`.
    pub fn namespace<'a>(&self, document: &'a Document) -> Option<&'a str> {
        self.namespace_for_prefix(document, self.prefix(document))
    }

    /// Gets HashMap of `prefix:namespace` declared in its attributes.
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
    /// Get namespace value given prefix, for this element.
    pub fn namespace_for_prefix<'a>(
        &self,
        document: &'a Document,
        prefix: &str,
    ) -> Option<&'a str> {
        let mut elem = *self;
        loop {
            let data = elem.data(document);
            if let Some(value) = data.namespace_decls.get(prefix) {
                return Some(value);
            }
            elem = elem.parent(document)?;
        }
    }

    pub fn parent(&self, document: &Document) -> Option<Element> {
        self.data(document).parent
    }

    /// ```ignore
    /// self.parent(document).is_some()
    /// ```
    pub fn has_parent(&self, document: &Document) -> bool {
        self.parent(document).is_some()
    }

    pub fn children<'a>(&self, document: &'a Document) -> &'a Vec<Node> {
        &self.data(document).children
    }

    /// ```ignore
    /// !self.children(document).is_empty()
    /// ```
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

    /// Equivalent to `vec.push()`.
    ///
    /// # Errors
    ///
    /// - [`Error::HasAParent`]: If node is an element, it must not have a parent.
    /// Call `elem.detatch()` before.
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

    /// Equivalent to `vec.insert()`.
    ///
    /// # Errors
    ///
    /// - [`Error::HasAParent`]: If node is an element, it must not have a parent.
    /// Call `elem.detatch()` before.
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

    /// Equivalent to `vec.remove()`.
    ///
    /// # Panics
    ///
    /// Panics if index is our of bounds.
    pub fn remove_child(&self, document: &mut Document, index: usize) -> Node {
        self.mut_data(document).children.remove(index)
    }

    /// Remove child element by value.
    ///
    /// # Errors
    ///
    /// - [Error::NotFound]: Element was not found among its children.
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

    pub fn detatch(&self, document: &mut Document) -> Result<()> {
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
