use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs::File;
use std::path::Path;

macro_rules! bench {
    ($filename:literal, $name:ident, $func:path) => {
        fn $name(c: &mut Criterion) {
            let path = Path::new($filename);
            c.bench_function(stringify!($name), |b| b.iter(|| $func(&path)));
        }
    };
}

fn xmldoc_parse(path: &Path) {
    let doc = xml_doc::Document::parse_file(path).unwrap();
    black_box(doc);
}
bench!("tiny.xml", tiny_xmldoc, xmldoc_parse);
bench!("medium.xml", medium_xmldoc, xmldoc_parse);
bench!("large.xml", large_xmldoc, xmldoc_parse);
bench!("medium_utf16.xml", utf16_xmldoc, xmldoc_parse);

fn minidom_parse(path: &Path) {
    let mut reader = minidom::quick_xml::Reader::from_file(path).unwrap();
    let doc = minidom::Element::from_reader(&mut reader).unwrap();
    black_box(doc);
}
bench!("tiny.xml", tiny_minidom, minidom_parse);
bench!("medium.xml", medium_minidom, minidom_parse);
bench!("large.xml", large_minidom, minidom_parse);

fn roxmltree_parse<'a>(path: &Path) {
    // roxmltree doesn't implement reading from reader.
    let xml = std::fs::read_to_string(path).unwrap();
    let doc = roxmltree::Document::parse(xml.as_ref()).unwrap();
    black_box(doc);
}
bench!("tiny.xml", tiny_roxmltree, roxmltree_parse);
bench!("medium.xml", medium_roxmltree, roxmltree_parse);
bench!("large.xml", large_roxmltree, roxmltree_parse);

fn xmltree_parse(path: &Path) {
    let file = File::open(path).unwrap();
    let doc = xmltree::Element::parse(file).unwrap();
    black_box(doc);
}
bench!("tiny.xml", tiny_xmltree, xmltree_parse);
bench!("medium.xml", medium_xmltree, xmltree_parse);
bench!("large.xml", large_xmltree, xmltree_parse);

criterion_group! {
    name = tiny;
    config = Criterion::default().sample_size(200);
    targets = tiny_xmldoc, tiny_minidom, tiny_roxmltree, tiny_xmltree
}

criterion_group!(
    medium,
    medium_xmldoc,
    medium_minidom,
    medium_roxmltree,
    medium_xmltree,
);

criterion_group! {
    name = large;
    config = Criterion::default().sample_size(50);
    targets = large_xmldoc, large_minidom, large_roxmltree, large_xmltree
}

criterion_group!(utf_16, utf16_xmldoc);

criterion_group!(
    xmldoc,
    tiny_xmldoc,
    medium_xmldoc,
    large_xmldoc,
    utf16_xmldoc
);
criterion_group!(roxmltree, tiny_roxmltree, medium_roxmltree, large_roxmltree);
criterion_group!(xmltree, tiny_xmltree, medium_xmltree, large_xmltree);
criterion_group!(minidom, tiny_minidom, medium_minidom, large_minidom);

criterion_main!(tiny, medium, large, utf_16);
