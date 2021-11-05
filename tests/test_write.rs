use xml_doc::{Document, Element, Node};

#[test]
fn test_escape() {
    let expected = r#"<?xml version="1.0" encoding="UTF-8"?>
<root attr="&gt;&lt;&amp;&quot;&apos;attrval">
  <inner xmlns:ns="&gt;&lt;&amp;&quot;&apos;nsval">&gt;&lt;&amp;&quot;&apos;text</inner>
</root>
<!--<&amp;--><![CDATA[<&amp;]]><!DOCTYPE &lt;&amp;amp;>
<?<&amp;?>"#;
    let mut doc = Document::new();
    let container = doc.container();
    let root = Element::build("root")
        .attribute("attr", "><&\"'attrval")
        .push_to(&mut doc, container);
    Element::build("inner")
        .namespace_decl("ns", "><&\"'nsval")
        .text_content("><&\"'text")
        .push_to(&mut doc, root);
    doc.push_root_node(Node::Comment("<&amp;".to_string()))
        .unwrap();
    doc.push_root_node(Node::CData("<&amp;".to_string()))
        .unwrap();
    doc.push_root_node(Node::DocType("<&amp;".to_string()))
        .unwrap();
    doc.push_root_node(Node::PI("<&amp;".to_string())).unwrap();
    let xml = doc.write_str().unwrap();

    assert_eq!(xml, expected);
}
