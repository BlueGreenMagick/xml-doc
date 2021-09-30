use crate::document::{Document, Node};
use crate::error::{Error, Result};
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct ElementData {
    full_name: String,
    attributes: HashMap<String, String>, // q:attr="val" => {"q:attr": "val"}
    namespace_decls: HashMap<String, String>, // local namespace newly defined in attributes
    parent: Option<Element>,
    children: Vec<Node>,
}

/// An easy way to build a new element
/// by chaining methods to add properties.
///
/// Call [`Element::build()`] to start building.
/// To finish building, either call `.finish()` or `.push_to(parent)`
/// which returns [`Element`].
///
/// # Examples
///
/// ```
/// use easy_xml::{Document, Element, Node};
///
/// let mut doc = Document::new();
///
/// let root = Element::build(&mut doc, "root")
///     .attribute("id", "main")
///     .attribute("class", "main")
///     .finish();
/// doc.push_new_node(root.as_node());
///
/// let name = Element::build(&mut doc, "name")
///     .text_content("No Name")
///     .push_to(root);
///
/// /* Equivalent xml:
///   <root id="main" class="main">
///     <name>No Name</name>
///   </root>
/// */
/// ```
///
pub struct ElementBuilder<'a> {
    element: Element,
    doc: &'a mut Document,
}

impl<'a> ElementBuilder<'a> {
    fn new(element: Element, doc: &'a mut Document) -> ElementBuilder<'a> {
        ElementBuilder { element, doc }
    }

    /// Removes previous prefix if it exists, and attach new prefix.
    pub fn prefix<S: Into<String>>(self, prefix: S) -> Self {
        self.element.set_prefix(self.doc, prefix);
        self
    }

    pub fn attribute<S, T>(self, name: S, value: T) -> Self
    where
        S: Into<String>,
        T: Into<String>,
    {
        self.element.set_attribute(self.doc, name, value);
        self
    }

    pub fn namespace_decl<S, T>(self, prefix: S, namespace: T) -> Self
    where
        S: Into<String>,
        T: Into<String>,
    {
        self.element.set_namespace_decl(self.doc, prefix, namespace);
        self
    }

    pub fn text_content<S: Into<String>>(self, text: S) -> Self {
        self.element.set_text_content(self.doc, text);
        self
    }

    pub fn finish(self) -> Element {
        self.element
    }

    pub fn push_to(self, parent: Element) -> Element {
        self.element.push_to(self.doc, parent).unwrap();
        self.element
    }
}

/// Represents an XML element.
///
/// This struct only contains a unique `usize` id and implements trait `Copy`.
/// So you do not need to bother with having a reference.
///
/// Because the actual data of the element is stored in [`Document`],
/// most methods takes `&Document` or `&mut Document` as its first argument.
///
/// Note that an element can only interact with elements of the same document.
/// If you for example attempt to call `.remove_child_elem()` with elements from other document,
/// unexpected errors may occur, or may panic.
/// You also can't move elements between documents.
///
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Element {
    id: usize,
}

impl Element {
    /// Create a new empty element with `full_name`.
    ///
    /// If full_name contains `:`,
    /// everything before that will be interpreted as a namespace prefix.
    pub fn new<S: Into<String>>(document: &mut Document, full_name: S) -> Self {
        Self::with_data(document, full_name.into(), HashMap::new(), HashMap::new())
    }

    /// Chain methods to build an element easily.
    pub fn build<S: Into<String>>(document: &mut Document, name: S) -> ElementBuilder {
        let element = Self::new(document, name);
        ElementBuilder::new(element, document)
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

    /// Create a container Element
    pub(crate) fn container() -> (Element, ElementData) {
        let elem_data = ElementData {
            full_name: String::new(),
            attributes: HashMap::new(),
            namespace_decls: HashMap::new(),
            parent: None,
            children: Vec::new(),
        };
        let elem = Element { id: 0 };
        (elem, elem_data)
    }

    /// Returns `true` if element is a container.
    ///
    /// See [`Document::container()`] for more information on 'container'.
    pub fn is_container(&self) -> bool {
        self.id == 0
    }

    pub fn as_node(&self) -> Node {
        Node::Element(*self)
    }

    /// Seperate full_name by `:`, returning (prefix, name).
    ///
    /// The first str is `""` if `full_name` has no prefix.
    pub fn separate_prefix_name(full_name: &str) -> (&str, &str) {
        match full_name.split_once(":") {
            Some((prefix, name)) => (prefix, name),
            None => ("", full_name),
        }
    }
}

/// Below are methods that take `&Document` as its first argument.
impl Element {
    fn data<'a>(&self, document: &'a Document) -> &'a ElementData {
        document.store.get(self.id).unwrap()
    }

    fn mut_data<'a>(&self, document: &'a mut Document) -> &'a mut ElementData {
        document.store.get_mut(self.id).unwrap()
    }

    /// Returns true if this element is the root node of document.
    ///
    /// Note that this crate allows Document to have multiple elements, even though it's not valid xml.
    pub fn is_root(&self, doc: &Document) -> bool {
        self.parent(doc).map_or(false, |p| p.is_container())
    }

    /// Get full name of element, including its namespace prefix.
    pub fn full_name<'a>(&self, document: &'a Document) -> &'a str {
        &self.data(document).full_name
    }

    pub fn set_full_name<S: Into<String>>(&self, document: &mut Document, name: S) {
        self.mut_data(document).full_name = name.into();
    }

    /// Get prefix and name of element.
    ///
    /// `<prefix: name` -> `("prefix", "name")`
    pub fn prefix_name<'a>(&self, document: &'a Document) -> (&'a str, &'a str) {
        Self::separate_prefix_name(self.full_name(document))
    }

    /// Get namespace prefix of element, without name.
    ///
    /// `<prefix:name>` -> `"prefix"`
    pub fn prefix<'a>(&self, document: &'a Document) -> &'a str {
        self.prefix_name(document).0
    }

    /// Set prefix of element, preserving its name.
    ///
    /// `prefix` should not have a `:`,
    /// or everything after `:` will be interpreted as part of element name.    
    pub fn set_prefix<S: Into<String>>(&self, document: &mut Document, prefix: S) {
        let data = self.mut_data(document);
        let (_, name) = Self::separate_prefix_name(&data.full_name);
        data.full_name = format!("{}:{}", prefix.into(), name);
    }

    /// Get name of element, without its namespace prefix.
    ///
    /// `<prefix:name>` -> `"name"`
    pub fn name<'a>(&self, document: &'a Document) -> &'a str {
        self.prefix_name(document).1
    }

    /// Set name of element, preserving its prefix.
    ///
    /// `name` should not have a `:`,
    /// or everything before `:` may be interpreted as namespace prefix.
    pub fn set_name<S: Into<String>>(&self, document: &mut Document, name: S) {
        let data = self.mut_data(document);
        let (prefix, _) = Self::separate_prefix_name(&data.full_name);
        if prefix == "" {
            data.full_name = name.into();
        } else {
            data.full_name = format!("{}:{}", prefix, name.into());
        }
    }

    /// Get attributes of element.
    ///
    /// The attribute names may have namespace prefix. To strip the prefix and only its name, call [`Element::separate_prefix_name`].
    /// ```
    /// use easy_xml::{Document, Element};
    ///
    /// let mut doc = Document::new();
    /// let element = Element::build(&mut doc, "name")
    ///     .attribute("id", "name")
    ///     .attribute("pre:name", "value")
    ///     .finish();
    ///
    /// let attrs = element.attributes(&doc);
    /// for (full_name, value) in attrs {
    ///     let (prefix, name) = Element::separate_prefix_name(full_name);
    ///     // ("", "id"), ("pre", "name")
    /// }
    /// ```
    pub fn attributes<'a>(&self, document: &'a Document) -> &'a HashMap<String, String> {
        &self.data(document).attributes
    }

    /// Add or set attribute.
    ///
    /// `name` should not contain a `:`,
    /// or everything before `:` will be interpreted as namespace prefix.
    pub fn set_attribute<S, T>(&self, document: &mut Document, name: S, value: T)
    where
        S: Into<String>,
        T: Into<String>,
    {
        self.mut_attributes(document)
            .insert(name.into(), value.into());
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

    /// Gets HashMap of `xmlns:prefix=namespace` declared in this element's attributes.
    pub fn namespace_decls<'a>(&self, document: &'a Document) -> &'a HashMap<String, String> {
        &self.data(document).namespace_decls
    }

    pub fn mut_namespace_decls<'a>(
        &self,
        document: &'a mut Document,
    ) -> &'a mut HashMap<String, String> {
        &mut self.mut_data(document).namespace_decls
    }

    pub fn set_namespace_decl<S, T>(&self, document: &mut Document, prefix: S, namespace: T)
    where
        S: Into<String>,
        T: Into<String>,
    {
        self.mut_namespace_decls(document)
            .insert(prefix.into(), namespace.into());
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

    pub(crate) fn build_text_content<'a>(&self, document: &'a Document, buf: &'a mut String) {
        for child in self.children(&document) {
            child.build_text_content(document, buf);
        }
    }

    /// Concatenate all text content of this element, including its child elements `text_content()`.
    ///
    /// Implementation of [Node.textContent](https://developer.mozilla.org/en-US/docs/Web/API/Node/textContent)
    pub fn text_content(&self, document: &Document) -> String {
        let mut buf = String::new();
        self.build_text_content(document, &mut buf);
        buf
    }

    /// Clears all its children and inserts a [`Node::Text`] with given text.
    pub fn set_text_content<S: Into<String>>(&self, document: &mut Document, text: S) {
        self.clear_children(document);
        let node = Node::Text(text.into());
        self.mut_data(document).children.push(node);
    }
}

/// Below are methods related to finding nodes in tree.
impl Element {
    pub fn parent(&self, document: &Document) -> Option<Element> {
        self.data(document).parent
    }

    /// `self.parent(document).is_some()`
    pub fn has_parent(&self, document: &Document) -> bool {
        self.parent(document).is_some()
    }

    /// Get child [`Node`]s of this element.
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

    /// Get all child nodes recursively. (i.e. includes its children's children.)
    pub fn children_recursive<'a>(&self, document: &'a Document) -> Vec<&'a Node> {
        let mut nodes = Vec::new();
        self._children_recursive(document, &mut nodes);
        nodes
    }

    /// `!self.children(document).is_empty()`
    pub fn has_children(&self, document: &Document) -> bool {
        !self.children(document).is_empty()
    }

    /// Get only child [`Element`]s of this element.
    ///
    /// This calls `.children().iter().filter_map().collect()`.
    /// Use [`Element::children()`] if performance is important.
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

    /// Get child [`Element`]s recursively. (i.e. includes its child element's child elements)
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

    /// Find first element with name `name`.
    pub fn find(&self, document: &Document, name: &str) -> Option<Element> {
        self.children(document)
            .iter()
            .filter_map(|n| n.as_element())
            .filter(|e| e.name(document) == name)
            .next()
    }

    /// Find first element with name `name`.
    /// If you care about performance, call `self.children().iter().filter()`
    pub fn find_all(&self, document: &Document, name: &str) -> Vec<Element> {
        self.children(document)
            .iter()
            .filter_map(|n| n.as_element())
            .filter(|e| e.name(document) == name)
            .collect()
    }
}

/// Below are functions that modify its tree-structure.
///
/// Because an element has reference to both its parent and its children,
/// an element's parent and children is not directly exposed for modification.
///
/// But in return, it is not possible for a document to be in an inconsistant state,
/// where an element's parent doesn't have the element as its children.
impl Element {
    /// Equivalent to `vec.push()`.
    ///
    /// # Errors
    ///
    /// - [`Error::HasAParent`]: If node is an element, it must not have a parent.
    /// Call `elem.detatch()` before.
    /// - [`Error::ContainerCannotMove`]: `node` cannot be container node.
    pub fn push_child(&self, document: &mut Document, node: Node) -> Result<()> {
        if let Node::Element(elem) = node {
            if elem.is_container() {
                return Err(Error::ContainerCannotMove);
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

    /// Equivalent to `parent.push_child()`.
    pub fn push_to(&self, document: &mut Document, parent: Element) -> Result<()> {
        parent.push_child(document, self.as_node())
    }

    /// Equivalent to `vec.insert()`.
    ///
    /// # Errors
    ///
    /// - [`Error::HasAParent`]: If node is an element, it must not have a parent.
    /// Call `elem.detatch()` before.
    /// - [`Error::ContainerCannotMove`]: `node` cannot be container node.
    pub fn insert_child(&self, document: &mut Document, index: usize, node: Node) -> Result<()> {
        if let Node::Element(elem) = node {
            if elem.is_container() {
                return Err(Error::ContainerCannotMove);
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
        let node = self.mut_data(document).children.remove(index);
        if let Node::Element(elem) = node {
            elem.mut_data(document).parent = None;
        }
        node
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

    pub fn clear_children(&self, document: &mut Document) {
        let children = &mut self.mut_data(document).children;
        for _ in 0..children.len() {
            self.remove_child(document, 0);
        }
    }

    /// Removes itself from its parent. Note that you can't add this element to other documents.
    ///
    /// # Errors
    ///
    /// - [`Error::ContainerCannotMove`]: You can't detatch container element
    pub fn detatch(&self, document: &mut Document) -> Result<()> {
        if self.is_container() {
            return Err(Error::ContainerCannotMove);
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
    use std::str::FromStr;

    #[test]
    fn test_children() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
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
        let outer = doc.container().child_elements(&doc)[0];
        let middle = outer.child_elements(&doc)[0];
        let inner = middle.child_elements(&doc)[0];
        let after = outer.child_elements(&doc)[1];
        assert_eq!(doc.container().child_elements(&doc).len(), 1);
        assert_eq!(outer.name(&doc), "outer");
        assert_eq!(middle.name(&doc), "middle");
        assert_eq!(inner.name(&doc), "inner");
        assert_eq!(after.name(&doc), "after");
        assert_eq!(outer.children(&doc).len(), 3);
        assert_eq!(outer.child_elements(&doc).len(), 2);
        assert_eq!(doc.container().children_recursive(&doc).len(), 8);
        assert_eq!(
            doc.container().child_elements_recursive(&doc),
            vec![outer, middle, inner, after]
        );
    }

    #[test]
    fn test_namespace() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
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
        let container = doc.container().children(&doc)[0].as_element().unwrap();
        let child_elements = container.child_elements(&doc);
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
        assert_eq!(container.namespace(&doc).unwrap(), "ns");
    }

    #[test]
    fn test_find_text_content() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <core>
            <p>Text</p>
            <b>Text2</b>
        </core>
        "#;
        let doc = Document::from_str(xml).unwrap();
        assert_eq!(
            doc.root_element()
                .unwrap()
                .find(&doc, "p")
                .unwrap()
                .text_content(&doc),
            "Text"
        );
        assert_eq!(
            doc.root_element()
                .unwrap()
                .find(&doc, "b")
                .unwrap()
                .text_content(&doc),
            "Text2"
        );
        assert_eq!(doc.root_element().unwrap().text_content(&doc), "TextText2")
    }
}
