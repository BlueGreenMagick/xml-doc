use super::error::{Error, Result};
use super::{Document, Node};
use std::collections::HashMap;

/// Represents a XML document.
#[derive(Debug)]
pub struct ElementData {
    full_name: String,
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
        full_name: String,
        attributes: HashMap<String, String>,
        namespace_decls: HashMap<String, String>,
    ) -> Element {
        let elem = Element {
            id: document.counter,
        };
        let elem_data = ElementData {
            full_name,
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
            full_name: String::new(),
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

    pub fn seperate_prefix_name<'a>(full_name: &'a str) -> (&'a str, &'a str) {
        match full_name.split_once(":") {
            Some((prefix, name)) => (prefix, name),
            None => ("", &full_name),
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
    pub fn full_name<'a>(&self, document: &'a Document) -> &'a str {
        &self.data(document).full_name
    }

    /// Get prefix and name of element.
    ///
    /// `<prefix: name` -> `("prefix", "name")`
    pub fn prefix_name<'a>(&self, document: &'a Document) -> (&'a str, &'a str) {
        Self::seperate_prefix_name(self.full_name(document))
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

    fn _children_recursive<'a>(&self, document: &'a Document, nodes: &mut Vec<&'a Node>) {
        for node in self.children(document) {
            nodes.push(node);
            if let Node::Element(elem) = &node {
                elem._children_recursive(document, nodes);
            }
        }
    }

    pub fn children_recursive<'a>(&self, document: &'a Document) -> Vec<&'a Node> {
        let mut nodes = Vec::new();
        self._children_recursive(document, &mut nodes);
        nodes
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

    pub fn child_elements_recursive(&self, document: &Document) -> Vec<Element> {
        self.children_recursive(document)
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

#[cfg(test)]
mod tests {
    use super::Document;

    #[test]
    fn test_children() {
        let xml = r#"
        <outer>
            inside outer
            <middle>
                <inner>
                    inside
                </inner>
                after inside
            </middle>
            <after>
                inside after
            </after>
        </outer>
        "#;
        let doc = Document::from_str(xml).unwrap();
        let outer = doc.root().child_elements(&doc)[0];
        let middle = outer.child_elements(&doc)[0];
        let inner = middle.child_elements(&doc)[0];
        let after = outer.child_elements(&doc)[1];
        assert_eq!(doc.root().child_elements(&doc).len(), 1);
        assert_eq!(outer.name(&doc), "outer");
        assert_eq!(middle.name(&doc), "middle");
        assert_eq!(inner.name(&doc), "inner");
        assert_eq!(after.name(&doc), "after");
        assert_eq!(outer.children(&doc).len(), 3);
        assert_eq!(outer.child_elements(&doc).len(), 2);
        assert_eq!(doc.root().children_recursive(&doc).len(), 8);
        assert_eq!(
            doc.root().child_elements_recursive(&doc),
            vec![outer, middle, inner, after]
        );
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
        let doc = Document::from_str(xml).unwrap();
        let root = doc.root().children(&doc)[0].as_element().unwrap();
        let child_elements = root.child_elements(&doc);
        let foo = *child_elements.get(0).unwrap();
        let bar = *child_elements.get(1).unwrap();
        let c = bar.child_elements(&doc)[0];
        assert_eq!(c.prefix_name(&doc), ("", "c"));
        assert_eq!(bar.full_name(&doc), "p:bar");
        assert_eq!(bar.prefix(&doc), "p");
        assert_eq!(bar.name(&doc), "bar");
        assert_eq!(c.namespace(&doc).unwrap(), "ns");
        assert_eq!(c.namespace_for_prefix(&doc, "p").unwrap(), "in2");
        assert!(c.namespace_for_prefix(&doc, "random").is_none());
        assert_eq!(bar.namespace(&doc).unwrap(), "in2");
        assert_eq!(bar.namespace_for_prefix(&doc, "").unwrap(), "ns");
        assert_eq!(foo.namespace(&doc).unwrap(), "pns");
        assert_eq!(foo.namespace_for_prefix(&doc, "").unwrap(), "inner");
        assert_eq!(foo.namespace_for_prefix(&doc, "p").unwrap(), "pns");
        assert_eq!(root.namespace(&doc).unwrap(), "ns");
    }
}
