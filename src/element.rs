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
/// use xml_doc::{Document, Element, Node};
///
/// let mut doc = Document::new();
///
/// let root = Element::build(&mut doc, "root")
///     .attribute("id", "main")
///     .attribute("class", "main")
///     .finish();
/// doc.push_root_node(root.as_node());
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

/// Represents an XML element. It acts as a pointer to actual element data stored in Document.
///
/// This struct only contains a unique `usize` id and implements trait `Copy`.
/// So you do not need to bother with having a reference.
///
/// Because the actual data of the element is stored in [`Document`],
/// most methods takes `&Document` or `&mut Document` as its first argument.
///
/// Note that an element may only interact with elements of the same document,
/// but the crate doesn't know which document an element is from.
///
/// If you for example attempt to call `.remove_child_elem()` with elements from other document,
/// unexpected errors may occur, or may panic.
/// You also can't move elements between documents.
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
    pub fn new<S: Into<String>>(doc: &mut Document, full_name: S) -> Self {
        Self::with_data(doc, full_name.into(), HashMap::new(), HashMap::new())
    }

    /// Chain methods to build an element easily.
    /// The chain can be finished with `.finish()` or `.push_to(parent)`.
    ///
    /// # Example
    /// ```
    /// use xml_doc::{Document, Element, Node};
    ///
    /// let mut doc = Document::new();
    ///
    /// let elem = Element::build(&mut doc, "root")
    ///     .attribute("id", "main")
    ///     .attribute("class", "main")
    ///     .finish();
    ///
    /// doc.push_root_node(elem.as_node());
    /// ```
    pub fn build<S: Into<String>>(doc: &mut Document, name: S) -> ElementBuilder {
        let element = Self::new(doc, name);
        ElementBuilder::new(element, doc)
    }

    pub(crate) fn with_data(
        doc: &mut Document,
        full_name: String,
        attributes: HashMap<String, String>,
        namespace_decls: HashMap<String, String>,
    ) -> Element {
        let elem = Element { id: doc.counter };
        let elem_data = ElementData {
            full_name,
            attributes,
            namespace_decls,
            parent: None,
            children: vec![],
        };
        doc.store.push(elem_data);
        doc.counter += 1;
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

    /// Equivalent to `Node::Element(self)`
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
    fn data<'a>(&self, doc: &'a Document) -> &'a ElementData {
        doc.store.get(self.id).unwrap()
    }

    fn mut_data<'a>(&self, doc: &'a mut Document) -> &'a mut ElementData {
        doc.store.get_mut(self.id).unwrap()
    }

    /// Returns true if this element is the root node of document.
    ///
    /// Note that this crate allows Document to have multiple elements, even though it's not valid xml.
    pub fn is_root(&self, doc: &Document) -> bool {
        self.parent(doc).map_or(false, |p| p.is_container())
    }

    /// Get full name of element, including its namespace prefix.
    /// Use [`Element::name()`] to get its name without the prefix.
    pub fn full_name<'a>(&self, doc: &'a Document) -> &'a str {
        &self.data(doc).full_name
    }

    pub fn set_full_name<S: Into<String>>(&self, doc: &mut Document, name: S) {
        self.mut_data(doc).full_name = name.into();
    }

    /// Get prefix and name of element.
    ///
    /// `<prefix: name` -> `("prefix", "name")`
    pub fn prefix_name<'a>(&self, doc: &'a Document) -> (&'a str, &'a str) {
        Self::separate_prefix_name(self.full_name(doc))
    }

    /// Get namespace prefix of element, without name.
    ///
    /// `<prefix:name>` -> `"prefix"`
    pub fn prefix<'a>(&self, doc: &'a Document) -> &'a str {
        self.prefix_name(doc).0
    }

    /// Set prefix of element, preserving its name.
    ///
    /// `prefix` should not have a `:`,
    /// or everything after `:` will be interpreted as part of element name.    
    ///
    /// If prefix is an empty string, removes prefix.
    pub fn set_prefix<S: Into<String>>(&self, doc: &mut Document, prefix: S) {
        let data = self.mut_data(doc);
        let (_, name) = Self::separate_prefix_name(&data.full_name);
        data.full_name = format!("{}:{}", prefix.into(), name);
    }

    /// Get name of element, without its namespace prefix.
    /// Use `Element::full_name()` to get its full name with prefix.
    ///
    /// `<prefix:name>` -> `"name"`
    pub fn name<'a>(&self, doc: &'a Document) -> &'a str {
        self.prefix_name(doc).1
    }

    /// Set name of element, preserving its prefix.
    ///
    /// `name` should not have a `:`,
    /// or everything before `:` may be interpreted as namespace prefix.
    pub fn set_name<S: Into<String>>(&self, doc: &mut Document, name: S) {
        let data = self.mut_data(doc);
        let (prefix, _) = Self::separate_prefix_name(&data.full_name);
        if prefix.is_empty() {
            data.full_name = name.into();
        } else {
            data.full_name = format!("{}:{}", prefix, name.into());
        }
    }

    /// Get attributes of element.
    ///
    /// The attribute names may have namespace prefix. To strip the prefix and only its name, call [`Element::separate_prefix_name`].
    /// ```
    /// use xml_doc::{Document, Element};
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
    pub fn attributes<'a>(&self, doc: &'a Document) -> &'a HashMap<String, String> {
        &self.data(doc).attributes
    }

    pub fn attribute<'a>(&self, doc: &'a Document, name: &str) -> Option<&'a str> {
        self.attributes(doc).get(name).map(|v| v.as_str())
    }

    /// Add or set attribute.
    ///
    /// If `name` contains a `:`,
    /// everything before `:` will be interpreted as namespace prefix.
    pub fn set_attribute<S, T>(&self, doc: &mut Document, name: S, value: T)
    where
        S: Into<String>,
        T: Into<String>,
    {
        self.mut_attributes(doc).insert(name.into(), value.into());
    }

    pub fn mut_attributes<'a>(&self, doc: &'a mut Document) -> &'a mut HashMap<String, String> {
        &mut self.mut_data(doc).attributes
    }

    /// Gets the namespace of this element.
    ///
    /// Shorthand for `self.namespace_for_prefix(doc, self.prefix(doc))`.
    pub fn namespace<'a>(&self, doc: &'a Document) -> Option<&'a str> {
        self.namespace_for_prefix(doc, self.prefix(doc))
    }

    /// Gets HashMap of `xmlns:prefix=namespace` declared in this element's attributes.
    ///
    /// Default namespace has empty string as key.
    pub fn namespace_decls<'a>(&self, doc: &'a Document) -> &'a HashMap<String, String> {
        &self.data(doc).namespace_decls
    }

    pub fn mut_namespace_decls<'a>(
        &self,
        doc: &'a mut Document,
    ) -> &'a mut HashMap<String, String> {
        &mut self.mut_data(doc).namespace_decls
    }

    pub fn set_namespace_decl<S, T>(&self, doc: &mut Document, prefix: S, namespace: T)
    where
        S: Into<String>,
        T: Into<String>,
    {
        self.mut_namespace_decls(doc)
            .insert(prefix.into(), namespace.into());
    }

    /// Get namespace value given prefix, for this element.
    /// "xml" and "xmlns" returns its default namespace.
    pub fn namespace_for_prefix<'a>(&self, doc: &'a Document, prefix: &str) -> Option<&'a str> {
        match prefix {
            "xml" => return Some("http://www.w3.org/XML/1998/namespace"),
            "xmlns" => return Some("http://www.w3.org/2000/xmlns/"),
            _ => (),
        };
        let mut elem = *self;
        loop {
            let data = elem.data(doc);
            if let Some(value) = data.namespace_decls.get(prefix) {
                return Some(value);
            }
            elem = elem.parent(doc)?;
        }
    }

    pub(crate) fn build_text_content<'a>(&self, doc: &'a Document, buf: &'a mut String) {
        for child in self.children(doc) {
            child.build_text_content(doc, buf);
        }
    }

    /// Concatenate all text content of this element, including its child elements `text_content()`.
    ///
    /// Implementation of [Node.textContent](https://developer.mozilla.org/en-US/docs/Web/API/Node/textContent)
    pub fn text_content(&self, doc: &Document) -> String {
        let mut buf = String::new();
        self.build_text_content(doc, &mut buf);
        buf
    }

    /// Clears all its children and inserts a [`Node::Text`] with given text.
    pub fn set_text_content<S: Into<String>>(&self, doc: &mut Document, text: S) {
        self.clear_children(doc);
        let node = Node::Text(text.into());
        self.mut_data(doc).children.push(node);
    }
}

/// Below are methods related to finding nodes in tree.
impl Element {
    pub fn parent(&self, doc: &Document) -> Option<Element> {
        self.data(doc).parent
    }

    /// `self.parent(doc).is_some()`
    pub fn has_parent(&self, doc: &Document) -> bool {
        self.parent(doc).is_some()
    }

    /// Get child [`Node`]s of this element.
    pub fn children<'a>(&self, doc: &'a Document) -> &'a Vec<Node> {
        &self.data(doc).children
    }

    fn _children_recursive<'a>(&self, doc: &'a Document, nodes: &mut Vec<&'a Node>) {
        for node in self.children(doc) {
            nodes.push(node);
            if let Node::Element(elem) = &node {
                elem._children_recursive(doc, nodes);
            }
        }
    }

    /// Get all child nodes recursively. (i.e. includes its children's children.)
    pub fn children_recursive<'a>(&self, doc: &'a Document) -> Vec<&'a Node> {
        let mut nodes = Vec::new();
        self._children_recursive(doc, &mut nodes);
        nodes
    }

    /// `!self.children(doc).is_empty()`
    pub fn has_children(&self, doc: &Document) -> bool {
        !self.children(doc).is_empty()
    }

    /// Get only child [`Element`]s of this element.
    ///
    /// This calls `.children().iter().filter_map().collect()`.
    /// Use [`Element::children()`] if performance is important.
    pub fn child_elements(&self, doc: &Document) -> Vec<Element> {
        self.children(doc)
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
    pub fn child_elements_recursive(&self, doc: &Document) -> Vec<Element> {
        self.children_recursive(doc)
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

    /// Find first direct child element with name `name`.
    pub fn find(&self, doc: &Document, name: &str) -> Option<Element> {
        self.children(doc)
            .iter()
            .filter_map(|n| n.as_element())
            .find(|e| e.name(doc) == name)
    }

    /// Find all direct child element with name `name`.
    /// If you care about performance, call `self.children().iter().filter()`
    pub fn find_all(&self, doc: &Document, name: &str) -> Vec<Element> {
        self.children(doc)
            .iter()
            .filter_map(|n| n.as_element())
            .filter(|e| e.name(doc) == name)
            .collect()
    }
}

/// Below are functions that modify its tree-structure.
///
/// Because an element has reference to both its parent and its children,
/// an element's parent and children is not directly exposed for modification.
/// But in return, it is not possible for a document to be in an inconsistant state,
/// where an element's parent doesn't have the element as its children.
///
/// # Errors
///
/// These errors are shared by below methods.
///
/// - [`Error::HasAParent`]: When you want to replace an element's parent with another,
/// call `element.detatch()` to make it parentless first.
/// This is to make it explicit that you are changing an element's parent, not adding another.
/// - [`Error::ContainerCannotMove`]: The container element's parent must always be None.
impl Element {
    /// Equivalent to `vec.push()`.
    pub fn push_child(&self, doc: &mut Document, node: Node) -> Result<()> {
        if let Node::Element(elem) = node {
            if elem.is_container() {
                return Err(Error::ContainerCannotMove);
            }
            let data = elem.mut_data(doc);
            if data.parent.is_some() {
                return Err(Error::HasAParent);
            }
            data.parent = Some(*self);
        }
        self.mut_data(doc).children.push(node);
        Ok(())
    }

    /// Equivalent to `parent.push_child()`.
    pub fn push_to(&self, doc: &mut Document, parent: Element) -> Result<()> {
        parent.push_child(doc, self.as_node())
    }

    /// Equivalent to `vec.insert()`.
    ///
    /// # Panics
    ///
    /// Panics if `index > self.children().len()`
    pub fn insert_child(&self, doc: &mut Document, index: usize, node: Node) -> Result<()> {
        if let Node::Element(elem) = node {
            if elem.is_container() {
                return Err(Error::ContainerCannotMove);
            }
            let data = elem.mut_data(doc);
            if data.parent.is_some() {
                return Err(Error::HasAParent);
            }
            data.parent = Some(*self);
        }
        self.mut_data(doc).children.insert(index, node);
        Ok(())
    }

    /// Equivalent to `vec.remove()`.
    ///
    /// # Panics
    ///
    /// Panics if `index >= self.children().len()`.
    pub fn remove_child(&self, doc: &mut Document, index: usize) -> Node {
        let node = self.mut_data(doc).children.remove(index);
        if let Node::Element(elem) = node {
            elem.mut_data(doc).parent = None;
        }
        node
    }

    /// Remove child element by value.
    ///
    /// # Errors
    ///
    /// - [Error::NotFound]: Element was not found among its children.
    pub fn remove_child_elem(&self, doc: &mut Document, element: Element) -> Result<()> {
        let children = &mut self.mut_data(doc).children;
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
        element.mut_data(doc).parent = None;
        Ok(())
    }

    pub fn pop_child(&self, doc: &mut Document) -> Option<Node> {
        let child = self.mut_data(doc).children.pop();
        if let Some(Node::Element(elem)) = &child {
            elem.mut_data(doc).parent = None;
        }
        child
    }

    /// Remove all children
    pub fn clear_children(&self, doc: &mut Document) {
        let children = &mut self.mut_data(doc).children;
        for _ in 0..children.len() {
            self.remove_child(doc, 0);
        }
    }

    /// Removes itself from its parent. Note that you can't attach this element to other documents.
    ///
    /// # Errors
    ///
    /// - [`Error::ContainerCannotMove`]: You can't detatch container element
    pub fn detatch(&self, doc: &mut Document) -> Result<()> {
        if self.is_container() {
            return Err(Error::ContainerCannotMove);
        }
        let parent = self.data(doc).parent;
        if let Some(parent) = parent {
            parent.remove_child_elem(doc, *self)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Document, Element, Node};

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
        let doc = Document::parse_str(xml).unwrap();
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
        let doc = Document::parse_str(xml).unwrap();
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
        let doc = Document::parse_str(xml).unwrap();
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

    #[test]
    fn test_mutate_tree() {
        // Test tree consistency after mutating tree
        let mut doc = Document::new();
        let container = doc.container();
        assert_eq!(container.parent(&doc), None);
        assert_eq!(container.children(&doc).len(), 0);

        // Element::build.push_to
        let root = Element::build(&mut doc, "root").push_to(container);
        assert_eq!(root.parent(&doc).unwrap(), container);
        assert_eq!(doc.root_element().unwrap(), root);

        // Element::new
        let a = Element::new(&mut doc, "a");
        assert_eq!(a.parent(&doc), None);

        // Element.push_child
        root.push_child(&mut doc, Node::Element(a)).unwrap();
        assert_eq!(root.children(&doc)[0].as_element().unwrap(), a);
        assert_eq!(a.parent(&doc).unwrap(), root);

        // Element.pop
        let popped = root.pop_child(&mut doc).unwrap().as_element().unwrap();
        assert_eq!(popped, a);
        assert_eq!(root.children(&doc).len(), 0);
        assert_eq!(a.parent(&doc), None);

        // Element.push_to
        let a = Element::new(&mut doc, "a");
        a.push_to(&mut doc, root).unwrap();
        assert_eq!(root.children(&doc)[0].as_element().unwrap(), a);
        assert_eq!(a.parent(&doc).unwrap(), root);

        // Element.remove_child_elem
        root.remove_child_elem(&mut doc, a).unwrap();
        assert_eq!(root.children(&doc).len(), 0);
        assert_eq!(a.parent(&doc), None);

        // Element.insert_child
        let a = Element::new(&mut doc, "a");
        root.insert_child(&mut doc, 0, Node::Element(a)).unwrap();
        assert_eq!(root.children(&doc)[0].as_element().unwrap(), a);
        assert_eq!(a.parent(&doc).unwrap(), root);

        // Element.remove_child
        root.remove_child(&mut doc, 0);
        assert_eq!(root.children(&doc).len(), 0);
        assert_eq!(a.parent(&doc), None);

        // Element.detatch
        let a = Element::build(&mut doc, "a").push_to(root);
        a.detatch(&mut doc).unwrap();
        assert_eq!(root.children(&doc).len(), 0);
        assert_eq!(a.parent(&doc), None);
    }
}
