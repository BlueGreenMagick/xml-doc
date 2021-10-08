use easy_xml::Document;

#[test]
fn test_normalize_attr() {
    // See comment on easy_xml::parser::DocumentParser::normalize_attr_value
    let xml = "<?xml version=\"1.0\"?>
<root attr=\" \r\t

 ab&#xD;   c
  \" />";
    let doc = Document::parse_str(xml).unwrap();
    let root = doc.root_element().unwrap();
    let val = root.attribute(&doc, "attr").unwrap();

    assert_eq!(val, "ab\r c");
}
