use xml_doc::{Document, Error, ReadOptions};

#[test]
fn test_normalize_attr() {
    // See comment on xml_doc::parser::DocumentParser::normalize_attr_value
    let xml = "<?xml version=\"1.0\"?>
<root attr=\" \r\t

 ab&#xD;   c
  \" />";
    let doc = Document::parse_str(xml).unwrap();
    let root = doc.root_element().unwrap();
    let val = root.attribute(&doc, "attr").unwrap();

    assert_eq!(val, "ab\r c");
}

#[test]
fn test_closing_tag_mismatch_err() {
    // no closing tag
    let xml = "<img>";
    let mut opts = ReadOptions::default();
    opts.require_decl = false;
    let doc = Document::parse_str_with_opts(xml, opts.clone());
    assert!(matches!(doc.unwrap_err(), Error::MalformedXML(_)));

    // closing tag mismatch
    let xml = "<a><img>Te</a>xt</img>";
    let doc = Document::parse_str_with_opts(xml, opts.clone());
    assert!(matches!(doc.unwrap_err(), Error::MalformedXML(_)));

    // no opening tag
    let xml = "</abc>";
    let doc = Document::parse_str_with_opts(xml, opts.clone());
    assert!(matches!(doc.unwrap_err(), Error::MalformedXML(_)));
}
