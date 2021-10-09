use xml_doc::{Document, Element};

#[test]
fn test_escape() {
    let expected = r#"<?xml version="1.0" encoding="UTF-8"?>
<root attr="&gt;&lt;&amp;&quot;&apos;attrval">
  <inner xmlns:ns="&gt;&lt;&amp;&quot;&apos;nsval">&gt;&lt;&amp;&quot;&apos;text</inner>
</root>"#;

    let mut doc = Document::new();
    let container = doc.container();
    let root = Element::build(&mut doc, "root")
        .attribute("attr", "><&\"'attrval")
        .push_to(container);
    Element::build(&mut doc, "inner")
        .namespace_decl("ns", "><&\"'nsval")
        .text_content("><&\"'text")
        .push_to(root);
    let xml = doc.write_str().unwrap();

    assert_eq!(xml, expected);
}
