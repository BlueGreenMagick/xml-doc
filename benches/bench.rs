use criterion::{criterion_group, criterion_main, Criterion};
use std::path::Path;
use std::str::FromStr;

macro_rules! bench {
    ($filename:literal, $name:ident, $func:path) => {
        fn $name(c: &mut Criterion) {
            let path = Path::new("benches").join($filename);
            let text = std::fs::read_to_string(path).unwrap();
            c.bench_function(stringify!($name), |b| b.iter(|| $func(&text).unwrap()));
        }
    };
}

bench!("tiny.xml", tiny_easyxml, easyxml::Document::from_str);
bench!("medium.xml", medium_easyxml, easyxml::Document::from_str);
bench!("large.xml", large_easyxml, easyxml::Document::from_str);

bench!("tiny.xml", tiny_minidom, minidom::Element::from_str);
bench!("medium.xml", medium_minidom, minidom::Element::from_str);
bench!("large.xml", large_minidom, minidom::Element::from_str);

bench!("tiny.xml", tiny_roxmltree, roxmltree::Document::parse);
bench!("medium.xml", medium_roxmltree, roxmltree::Document::parse);
bench!("large.xml", large_roxmltree, roxmltree::Document::parse);

fn xmltree_parse(text: &str) -> Result<xmltree::Element, xmltree::ParseError> {
    xmltree::Element::parse(text.as_bytes())
}
bench!("tiny.xml", tiny_xmltree, xmltree_parse);
bench!("medium.xml", medium_xmltree, xmltree_parse);
bench!("large.xml", large_xmltree, xmltree_parse);

bench!("tiny.xml", tiny_sdx_document, sxd_document::parser::parse);
bench!(
    "medium.xml",
    medium_sdx_document,
    sxd_document::parser::parse
);
bench!("large.xml", large_sdx_document, sxd_document::parser::parse);

bench!("tiny.xml", tiny_xml_dom, xml_dom::parser::read_xml);
bench!("medium.xml", medium_xml_dom, xml_dom::parser::read_xml);
// Does not manage to read the file.
// bench!("large.xml", large_xml_dom, xml_dom::parser::read_xml);

criterion_group! {
    name = tiny;
    config = Criterion::default().sample_size(200);
    targets = tiny_easyxml, tiny_minidom, tiny_roxmltree, tiny_xmltree, tiny_sdx_document, tiny_xml_dom
}

criterion_group!(
    medium,
    medium_easyxml,
    medium_minidom,
    medium_roxmltree,
    medium_xmltree,
    medium_sdx_document,
    medium_xml_dom
);

criterion_group! {
    name = large;
    config = Criterion::default().sample_size(50);
    targets = large_easyxml, large_minidom, large_roxmltree, large_xmltree, large_sdx_document
}

criterion_group!(easyxml, tiny_easyxml, medium_easyxml, large_easyxml);
criterion_group!(roxmltree, tiny_roxmltree, medium_roxmltree, large_roxmltree);
criterion_group!(xmltree, tiny_xmltree, medium_xmltree, large_xmltree);
criterion_group!(
    sdx,
    tiny_sdx_document,
    medium_sdx_document,
    large_sdx_document,
);
criterion_group!(minidom, tiny_minidom, medium_minidom, large_minidom);
criterion_group!(xml_dom, tiny_xml_dom, medium_xml_dom);

criterion_main!(xml_dom);
