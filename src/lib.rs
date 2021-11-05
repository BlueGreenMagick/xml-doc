//! A tree-like parser to read, modify and write XML files.
//!
//! It was especially designed for modifying xml files without rigid structure.
//!
//! Parsing from various encodings are supported, including UTF-16, ISO 8859-1, GBK and EUC-KR. (With the notable exception of UTF-32)
//!
//! The XML document is represented with [`Document`], [`Element`] and [`Node`].
//!
//!
//! # Example
//! ```
//! use xml_doc::{Document, Element, Node};
//!
//! const data: &'static str = r#"<?xml version="1.0" encoding="utf-8"?>
//! <metadata>     
//!     <title>The Felloship of the Ring</title>
//!     <author>J. R. R. Tolkien</author>
//!     <date>1954</date>
//! </metadata>
//! "#;
//!
//! let mut doc = Document::parse_str(data).unwrap();
//! let metadata = doc.root_element().unwrap();
//!
//! // Add a new element
//! let series = Element::build("series")
//!     .text_content("Lord of the Rings")
//!     .push_to(&mut doc, metadata);
//!
//! // Modify existing element
//! let date = metadata.find(&doc, "date").unwrap();
//! date.set_text_content(&mut doc, "29 July 1954");
//!
//! let xml = doc.write_str();
//! ```
//!
//! Below example goes through the root element's children and removes all nodes that isn't `<conf>...</conf>`
//! ```no_run
//! use std::path::Path;
//! use xml_doc::{Document, Node};
//!
//! let xml_file = Path::new("config.xml");
//! let mut doc = Document::parse_file(&xml_file).unwrap();
//! let root = doc.root_element().unwrap();
//! let to_remove: Vec<usize> = root.children(&doc)
//!     .iter()
//!     .enumerate()
//!     .filter_map(|(i, node)| {
//!         if let Node::Element(elem) = node {
//!             if elem.name(&doc) == "conf" {
//!                 return None
//!             }
//!         }
//!         Some(i)
//!     })
//!     .collect();
//! for i in to_remove.iter().rev() {
//!     root.remove_child(&mut doc, *i);
//! }
//! doc.write_file(&xml_file);
//! ```
//!
mod document;
mod element;
mod error;
mod parser;

pub use crate::document::{Document, Node, WriteOptions};
pub use crate::element::{Element, ElementBuilder};
pub use crate::error::{Error, Result};
pub use crate::parser::{normalize_space, ReadOptions};
