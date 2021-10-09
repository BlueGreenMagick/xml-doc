use xml_doc::Document;

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
