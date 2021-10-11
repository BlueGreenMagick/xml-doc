use xml_doc::{Document, Error, Node, ReadOptions};

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

#[test]
fn test_unescape() {
    let xml = r#"<abc attr="&quot;val&quot;">&lt;Text&amp;&apos;&gt;</abc>
    <![CDATA[<&amp;>]]>
    <!-- <&amp; cmt -->
    <!DOCTYPE &amp;>
    <?<&amp;?>"#;
    let mut opts = ReadOptions::default();
    opts.require_decl = false;
    let doc = Document::parse_str_with_opts(xml, opts).unwrap();

    let abc = doc.root_element().unwrap();
    assert_eq!(abc.attribute(&doc, "attr"), Some("\"val\""));
    let text = &abc.children(&doc)[0];
    assert!(matches!(text, Node::Text(_)));
    assert_eq!(text.text_content(&doc), "<Text&'>");

    let cdata = &doc.root_nodes()[1];
    assert!(matches!(cdata, Node::CData(_)));
    assert_eq!(cdata.text_content(&doc), "<&amp;>");

    let comment = &doc.root_nodes()[2];
    if let Node::Comment(cmt) = comment {
        assert_eq!(cmt, " <&amp; cmt ");
    } else {
        assert!(false);
    }

    let doctype = &doc.root_nodes()[3];
    if let Node::DocType(doc) = doctype {
        assert_eq!(doc, "&");
    } else {
        assert!(false);
    }

    let pi = &doc.root_nodes()[4];
    assert!(matches!(pi, Node::PI(_)));
    assert_eq!(pi.text_content(&doc), "<&amp;");
}
