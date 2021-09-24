use bencher::Bencher;
use bencher::{benchmark_group, benchmark_main};
use std::path::Path;
use std::str::FromStr;

macro_rules! bench {
    ($filename:literal, $name:ident, $func:path) => {
        fn $name(bencher: &mut Bencher) {
            let path = Path::new("benches").join($filename);
            let text = std::fs::read_to_string(path).unwrap();
            bencher.iter(|| $func(&text).unwrap())
        }
    };
}

bench!(
    "tiny.xml",
    tiny_quickxmltree,
    quick_xml_tree::Document::from_str
);
bench!(
    "medium.xml",
    medium_quickxmltree,
    quick_xml_tree::Document::from_str
);
bench!(
    "large.xml",
    large_quickxmltree,
    quick_xml_tree::Document::from_str
);

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

bench!("tiny.xml", tiny_minidom, minidom::Element::from_str);
bench!("medium.xml", medium_minidom, minidom::Element::from_str);
bench!("large.xml", large_minidom, minidom::Element::from_str);

benchmark_group!(
    quickxmltree,
    tiny_quickxmltree,
    medium_quickxmltree,
    large_quickxmltree
);
benchmark_group!(roxmltree, tiny_roxmltree, medium_roxmltree, large_roxmltree);
benchmark_group!(xmltree, tiny_xmltree, medium_xmltree, large_xmltree);
benchmark_group!(
    sdx,
    tiny_sdx_document,
    medium_sdx_document,
    large_sdx_document,
);
benchmark_group!(minidom, tiny_minidom, medium_minidom, large_minidom);

benchmark_main!(quickxmltree, roxmltree, xmltree, sdx, minidom);
