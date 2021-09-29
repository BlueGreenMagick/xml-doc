use easy_xml::{Document, Element, Node, ReadOptions};
use itertools::Itertools;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Write;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::str::FromStr;

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
    let container = document.container();
    render_nodes(document, container.children(&document), depth, &mut buf);
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
        }
    }
}

fn render_element(doc: &Document, elem: Element, mut depth: usize, buf: &mut String) {
    write_line("- Element:", depth, buf);
    depth += 2;

    let name = elem.full_name(doc);
    write_line(&format!("name: {}", name), depth, buf);

    let attrs = elem.attributes(doc);
    if attrs.len() > 0 {
        write_line("attributes:", depth, buf);
        write_hashmap_alphabetical(attrs, depth, buf);
    }

    let namespaces = elem.namespace_declarations(doc);
    if namespaces.len() > 0 {
        write_line("namespaces:", depth, buf);
        write_hashmap_alphabetical(namespaces, depth, buf);
    }
    let children = elem.children(doc);
    if children.len() > 0 {
        write_line("children:", depth, buf);
        depth += 1;
        render_nodes(doc, children, depth, buf);
    }
}

fn write_hashmap_alphabetical(map: &HashMap<String, String>, depth: usize, buf: &mut String) {
    let mut entries = Vec::new();
    for (key, val) in map.iter() {
        entries.push((key, val))
    }
    entries.sort_by_key(|x| x.0);
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

// Documents and xml files are supposed to have a 1:1 relationship.
// Then write is ok if read function is ok, and read(write(D)) == D
fn test_write(document: &Document) -> TStr {
    let expected = TStr(to_yaml(&document));
    let written_xml = document.write_str().unwrap();
    println!("{:?}", &written_xml);
    let new_doc = Document::from_str(&written_xml).unwrap();
    let result = TStr(to_yaml(&new_doc));
    assert!(
        expected == result,
        "\n===expected==={:?}\n===result==={:?}\nWRITING\n",
        expected,
        result,
    );
    expected
}

fn test<F, S>(xml_file: &str, expected: F)
where
    F: Fn(&ReadOptions) -> S,
    S: Into<String>,
{
    let xml_file = Path::new("tests/documents").join(xml_file);

    // Options
    let empty_text_node_opts = [true, false];
    let opts = [empty_text_node_opts];

    for k in opts.iter().multi_cartesian_product() {
        let read_options = ReadOptions {
            empty_text_node: *k[0],
        };
        let expected_name: String = expected(&read_options).into();
        let expected = get_expected(&expected_name);
        // Read xml document
        let mut document = Document::new();
        document.read_opts = read_options.clone();
        let file = File::open(&xml_file).unwrap();
        let reader = BufReader::new(file);
        let result = if let Err(error) = document.parse_reader(reader) {
            println!("{:?}", error);
            let debug_str = format!("{:?}", error);
            let variant_name = debug_str.splitn(2, "(").next().unwrap();
            TStr(format!("error: {}", variant_name))
        } else {
            test_write(&document)
        };

        assert!(
            expected == result,
            "\noptions: {:?}\n===expected==={:?}===result==={:?}\nREADING\n",
            read_options,
            expected,
            result,
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
fn encoding1() {
    test("encoding1.xml", |_| "encoding1.yaml".to_string())
}

#[test]
fn encoding2() {
    test("encoding2.xml", |_| "encoding2.yaml".to_string())
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
    test("standalone.xml", |_| "standalone_err.yaml".to_string())
}
