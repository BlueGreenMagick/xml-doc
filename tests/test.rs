use easyxml::{Document, ElementId, Node, ReadOptions};
use itertools::Itertools;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Write;
use std::path::Path;

#[derive(Clone)]
struct TStr(pub String);

impl PartialEq<Self> for TStr {
    fn eq(&self, other: &Self) -> bool {
        self.0.trim() == other.0.trim()
    }
}

impl<'a> fmt::Debug for TStr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n{}\n", self.0.trim())
    }
}

fn to_yaml(document: &Document) -> String {
    let mut buf = String::new();
    let mut depth: usize = 0;
    write_line("Root:", depth, &mut buf);
    depth += 1;
    let root_node = document.get_element(0).unwrap();
    render_nodes(document, root_node.get_children(), depth, &mut buf);
    buf
}

fn render_nodes(doc: &Document, nodes: &Vec<Node>, depth: usize, buf: &mut String) {
    for node in nodes {
        match node {
            Node::Element(id) => render_element(doc, *id, depth, buf),
            Node::Text(text) => write_line(&format!("- Text: \"{}\"", text), depth, buf),
            Node::Comment(text) => write_line(&format!("- Comment: \"{}\"", text), depth, buf),
            Node::CData(text) => write_line(&format!("- CData: \"{}\"", text), depth, buf),
            Node::DocType(text) => write_line(&format!("- DocType: \"{}\"", text), depth, buf),
            Node::PI(text) => write_line(&format!("- PI: \"{}\"", text), depth, buf),
            Node::Decl {
                version,
                encoding,
                standalone,
            } => {
                write_line("- Decl:", depth, buf);
                write_line(&format!("version: {}", version), depth + 2, buf);
                if let Some(val) = encoding {
                    write_line(&format!("encoding: {}", val), depth + 2, buf);
                }
                if let Some(val) = standalone {
                    write_line(&format!("encoding: {}", val), depth + 2, buf);
                }
            }
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
    let children = elem.get_children();
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

// main test functions
//////////////////////

fn get_expected(file_name: &str) -> TStr {
    let yaml_file = Path::new("tests/documents").join(file_name);

    TStr(
        std::fs::read_to_string(&yaml_file)
            .unwrap()
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<&str>>()
            .join("\n"),
    )
}

fn test<F, S>(xml_file: &str, expected: F)
where
    F: Fn(&ReadOptions) -> S,
    S: Into<String>,
{
    let xml_file = Path::new("tests/documents").join(xml_file);
    let xml_raw = std::fs::read_to_string(&xml_file).unwrap();

    // Options
    let standalone_opts = [true, false];
    let empty_text_node_opts = [true, false];
    let opts = [standalone_opts, empty_text_node_opts];

    for k in opts.iter().multi_cartesian_product() {
        let read_options = ReadOptions {
            standalone: *k[0],
            empty_text_node: *k[1],
        };
        let expected_name: String = expected(&read_options).into();
        let expected = get_expected(&expected_name);
        // Read xml document
        let mut document = Document::new();
        document.read_opts = read_options.clone();
        let result = if let Err(error) = document.read_str(&xml_raw) {
            let debug_str = format!("{:?}", error);
            let variant_name = debug_str.splitn(2, "(").next().unwrap();
            TStr(format!("error: {}", variant_name))
        } else {
            TStr(to_yaml(&document))
        };

        assert!(
            result == expected,
            "\noptions: {:?}\n===result==={:?}===expected==={:?}\n",
            read_options,
            result,
            expected,
        );
    }
}
#[test]
fn basic() {
    test("basic.xml", |_| "basic.yaml".to_string())
}

#[test]
fn emptytag() {
    test("emptytag.xml", |opts| {
        if opts.empty_text_node == true {
            "emptytag_emptytext.yaml"
        } else {
            "emptytag.yaml"
        }
    })
}

#[test]
fn error1() {
    test("error1.xml", |_| "error1.yaml".to_string())
}

#[test]
fn error2() {
    test("error2.xml", |_| "error2.yaml".to_string())
}

#[test]
fn nodes() {
    test("nodes.xml", |_| "nodes.yaml".to_string())
}
#[test]
fn namespace() {
    test("namespace.xml", |_| "namespace.yaml".to_string())
}

#[test]
fn standalone() {
    test("standalone.xml", |opts| {
        if opts.standalone == true {
            "standalone.yaml"
        } else {
            "standalone_err.yaml"
        }
    })
}
