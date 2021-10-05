//! A tree-like parser to read, modify and write XML files.
//!
//! It was especially designed for modifying xml files without rigid structure.
//!
//! Reading from various encodings are supported, including UTF-16, ISO 8859-1, GBK and EUC-KR. (With the notable exception of UTF-32)
//!
//! # Example
//! ```
//! use easy_xml::{Document, Element, Node};
//!
//! const data: &'static str = r#"<?xml version="1.0" encoding="utf-8"?>
//! <metadata>     
//!     <title>The Felloship of the Ring</title>
//!     <author>J. R. R. Tolkien</author>
//!     <date>1954</date>
//! </metadata>
//! "#;
//!
//! let mut doc = Document::new();
//! doc.parse_str(data);
//! let metadata = doc.root_element().unwrap();
//!
//! // Add a new element
//! let series = Element::new(&mut doc, "series");
//! series.set_text_content(&mut doc, "Lord of the Rings");
//! metadata.push_child(&mut doc, Node::Element(series));
//!
//! // Modify existing element
//! let date = metadata.find(&doc, "date").unwrap();
//! date.set_text_content(&mut doc, "29 July 1954");
//!
//! let xml = doc.write_str();
//! ```

mod document;
mod element;
mod error;
mod parser;

pub use crate::document::{Document, Node, ReadOptions};
pub use crate::element::{Element, ElementBuilder};
pub use crate::error::{Error, Result};
