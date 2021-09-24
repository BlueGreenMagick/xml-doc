use quick_xml_tree::{Document, Element, ElementId, Node};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Write;
use std::path::Path;

#[derive(Clone, PartialEq)]
struct TStr<'a>(pub &'a str);

impl<'a> fmt::Debug for TStr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n{}\n", self.0)
    }
}

fn to_yaml(document: &Document) -> String {
    let mut buf = String::new();
    let mut depth: usize = 0;
    write_line("Root:", depth, &mut buf);
    depth += 1;
    let root_node = document.get_element(0).unwrap();
    render_nodes(document, &root_node.children, depth, &mut buf);
    buf
}

fn render_nodes(doc: &Document, nodes: &Vec<Node>, depth: usize, buf: &mut String) {
    for node in nodes {
        match node {
            Node::Element(id) => render_element(doc, *id, depth, buf),
            Node::Text(text) => write_line(&format!("- Text: \"{}\"", text), depth, buf),
        }
    }
}

fn render_element(doc: &Document, id: ElementId, mut depth: usize, buf: &mut String) {
    let elem = doc.get_element(id).unwrap();
    write_line("- Element:", depth, buf);
    depth += 2;

    if let Some(prefix) = &elem.prefix {
        write_line(&format!("prefix: {}", prefix), depth, buf);
    }
    let name = &elem.name;
    write_line(&format!("name: {}", name), depth, buf);

    let attrs = &elem.attributes;
    if attrs.len() > 0 {
        write_line("attributes:", depth, buf);
        write_hashmap_alphabetical(attrs, depth, buf);
    }

    let namespaces = &elem.namespaces;
    if namespaces.len() > 0 {
        write_line("namespaces:", depth, buf);
        write_hashmap_alphabetical(namespaces, depth, buf);
    }
    let children = &elem.children;
    if children.len() > 0 {
        write_line("children:", depth, buf);
        depth += 1;
        render_nodes(doc, children, depth, buf);
    }
}

fn write_hashmap_alphabetical(map: &HashMap<String, String>, depth: usize, buf: &mut String) {
    let mut entries = Vec::new();
    for (key, val) in map.iter() {
        entries.push((key.clone(), val.clone()))
    }
    entries.sort_by_cached_key(|x| x.0.clone());
    for entry in entries {
        write_line(&format!("{}: \"{}\"", entry.0, entry.1), depth + 1, buf);
    }
}

fn write_line(text: &str, depth: usize, buf: &mut String) {
    let indent = " ".repeat(depth * 2);
    writeln!(buf, "{}{}", indent, text).unwrap();
}

fn test(file_name: &str) {
    let path = Path::new("tests/documents").join(file_name);
    let yaml_file = path.with_extension("yaml");
    let xml_file = path.with_extension("xml");
    let expected: String = std::fs::read_to_string(&yaml_file)
        .unwrap()
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<&str>>()
        .join("\n");
    let xml_raw_str = std::fs::read_to_string(&xml_file).unwrap();
    let result = match Document::from_str(&xml_raw_str) {
        Ok(document) => to_yaml(&document),
        Err(error) => {
            let debug_str = format!("{:?}", error);
            let variant_name = debug_str.splitn(2, "(").next().unwrap();
            format!("error: {}", variant_name)
        }
    };
    assert_eq!(TStr(result.trim()), TStr(expected.trim()));
}

macro_rules! test {
    ($name:ident) => {
        #[test]
        fn $name() {
            test(stringify!($name));
        }
    };
}

test!(basic1);
test!(basic2);
test!(basic3);
test!(basic4);
test!(basic5);
test!(basic6);
test!(standalone1);
test!(standalone2);
test!(error1);
test!(error2);
