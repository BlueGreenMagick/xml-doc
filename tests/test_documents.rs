use itertools::Itertools;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Write;
use std::path::Path;
use std::str::FromStr;
use xml_doc::{Document, Element, Node, ReadOptions};

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

fn to_yaml(doc: &Document) -> String {
    let mut buf = String::new();
    let mut depth: usize = 0;
    write_line("Root:", depth, &mut buf);
    depth += 1;
    let container = doc.container();
    render_nodes(doc, container.children(&doc), depth, &mut buf);
    buf
}

fn render_nodes(doc: &Document, nodes: &Vec<Node>, depth: usize, buf: &mut String) {
    for node in nodes {
        match node {
            Node::Element(id) => render_element(doc, *id, depth, buf),
            Node::Text(text) => write_line(
                &format!(
                    "- Text: \"{}\"",
                    text.replace("\n", r"\n").replace("\r", r"\r")
                ),
                depth,
                buf,
            ),
            Node::Comment(text) => write_line(
                &format!(
                    "- Comment: \"{}\"",
                    text.replace("\n", r"\n").replace("\r", r"\r")
                ),
                depth,
                buf,
            ),
            Node::CData(text) => write_line(
                &format!(
                    "- CData: \"{}\"",
                    text.replace("\n", r"\n").replace("\r", r"\r")
                ),
                depth,
                buf,
            ),
            Node::DocType(text) => write_line(
                &format!(
                    "- DocType: \"{}\"",
                    text.replace("\n", r"\n").replace("\r", r"\r")
                ),
                depth,
                buf,
            ),
            Node::PI(text) => write_line(
                &format!(
                    "- PI: \"{}\"",
                    text.replace("\n", r"\n").replace("\r", r"\r")
                ),
                depth,
                buf,
            ),
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

    let namespaces = elem.namespace_decls(doc);
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

// When read_opts and write_opts are both default,
// read(write(doc)) should be doc.
// just a basic test for writing.
fn test_write(doc: &Document) -> TStr {
    let expected = TStr(to_yaml(&doc));
    let written_xml = doc.write_str().unwrap();
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
    let trim_text = [true, false];
    let ignore_whitespace_only = [true, false];
    let require_decl = [true, false];
    let opts = [
        empty_text_node_opts,
        trim_text,
        ignore_whitespace_only,
        require_decl,
    ];

    for k in opts.iter().multi_cartesian_product() {
        let mut read_options = ReadOptions::default();
        read_options.empty_text_node = *k[0];
        read_options.trim_text = *k[1];
        read_options.ignore_whitespace_only = *k[2];
        read_options.require_decl = *k[3];
        let expected_name: String = expected(&read_options).into();
        let expected = get_expected(&expected_name);

        let result = match Document::parse_file_with_opts(&xml_file, read_options.clone()) {
            Ok(doc) => TStr(to_yaml(&doc)),
            Err(error) => {
                println!("{:?}", error);
                let debug_str = format!("{:?}", error);
                let variant_name = debug_str.splitn(2, "(").next().unwrap();
                TStr(format!("error: {}", variant_name))
            }
        };

        assert!(
            expected == result,
            "\noptions: {:?}\n===expected==={:?}===result==={:?}\nREADING\n",
            read_options,
            expected,
            result,
        );
    }
    // Test write
    let doc = Document::parse_file(&xml_file).unwrap();
    test_write(&doc);
}

#[test]
fn nodes() {
    test("nodes.xml", |opts| {
        if !opts.ignore_whitespace_only && !opts.trim_text {
            "nodes_noignws.yaml"
        } else if !opts.trim_text {
            "nodes_notrim.yaml"
        } else {
            "nodes_.yaml"
        }
    })
}

fn expected_doc_yaml<'a>(opts: &ReadOptions) -> &'a str {
    if opts.empty_text_node {
        if opts.trim_text {
            "doc_etn_trim.yaml"
        } else if opts.ignore_whitespace_only {
            "doc_etn_ignws.yaml"
        } else {
            "doc_etn.yaml"
        }
    } else if opts.trim_text {
        "doc_trim.yaml"
    } else if opts.ignore_whitespace_only {
        "doc_ignws.yaml"
    } else {
        "doc_.yaml"
    }
}
#[test]
fn document() {
    test("doc.xml", expected_doc_yaml)
}

#[test]
fn encoding1() {
    test("encoding1.xml", expected_doc_yaml)
}

#[test]
fn encoding2() {
    test("encoding2.xml", expected_doc_yaml)
}
