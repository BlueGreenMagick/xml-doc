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

/// Parser Benchmarking
/// Calculates sum of tag local name byte length and attribute values byte length
///
/// Because quick_xml just stores the whole tag content as bytes without processing it
/// the below test should take that into consideration as it makes it calculate tag name and attributes.
//////////////////////////////////////////

fn quick_xml_parser(path: &Path) -> usize {
    let mut count = 0;
    let mut reader = quick_xml::Reader::from_file(path).unwrap();
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf) {
            Ok(quick_xml::events::Event::Start(tag)) | Ok(quick_xml::events::Event::Empty(tag)) => {
                count += tag.name().splitn(1, |b| *b == b':').last().unwrap().len();
                for attr in tag.attributes() {
                    let attr = attr.unwrap();
                    count += attr.value.len();
                }
            }
            Ok(quick_xml::events::Event::Eof) => {
                break;
            }
            _ => (),
        }
    }
    count
}

bench!("tiny.xml", tiny_quick_xml, quick_xml_parser);
bench!("medium.xml", medium_quick_xml, quick_xml_parser);
bench!("large.xml", large_quick_xml, quick_xml_parser);
criterion_group!(quick_xml, tiny_quick_xml, medium_quick_xml, large_quick_xml);

mod xml5ever_bench {
    use super::*;
    use markup5ever::buffer_queue::BufferQueue;
    use xml5ever::tendril::*;
    use xml5ever::tokenizer::{TagToken, Token, TokenSink, XmlTokenizer};
    #[derive(Copy, Clone)]
    struct TokenCounter {
        counter: usize,
    }

    impl TokenSink for TokenCounter {
        fn process_token(&mut self, token: Token) {
            match token {
                TagToken(tag) => {
                    let name = tag.name.local.as_ref().len();
                    self.counter += name;
                    for attr in tag.attrs {
                        self.counter += attr.value.len();
                    }
                }
                _ => (),
            }
        }
    }

    pub fn xml5ever_parser(path: &Path) -> usize {
        let sink = TokenCounter { counter: 0 };

        let mut file = File::open(&path).ok().expect("can't open file");
        let mut input = ByteTendril::new();
        file.read_to_tendril(&mut input)
            .ok()
            .expect("can't read file");
        let mut input_buffer = BufferQueue::new();
        input_buffer.push_back(input.try_reinterpret().unwrap());
        let mut tok = XmlTokenizer::new(sink, Default::default());
        tok.feed(&mut input_buffer);
        tok.end();

        return sink.counter;
    }
}
use xml5ever_bench::xml5ever_parser;

bench!("tiny.xml", tiny_xml5ever, xml5ever_parser);
bench!("medium.xml", medium_xml5ever, xml5ever_parser);
bench!("large.xml", large_xml5ever, xml5ever_parser);
criterion_group!(xml5ever, tiny_xml5ever, medium_xml5ever, large_xml5ever);

fn rustyxml_parser(path: &Path) -> usize {
    let mut counter = 0;
    let xml = std::fs::read_to_string(path).unwrap();
    let mut parser = RustyXML::Parser::new();
    parser.feed_str(&xml);
    for event in parser {
        match event {
            Ok(RustyXML::Event::ElementStart(tag)) => {
                counter += tag.name.len();
                for (_, value) in tag.attributes {
                    counter += value.len();
                }
            }
            _ => (),
        }
    }
    counter
}

bench!("tiny.xml", tiny_rustyxml, rustyxml_parser);
bench!("medium.xml", medium_rustyxml, rustyxml_parser);
bench!("large.xml", large_rustyxml, rustyxml_parser);
criterion_group!(rustyxml, tiny_rustyxml, medium_rustyxml, large_rustyxml);
mod xml_rs_bench {
    use super::*;
    use std::io::BufReader;
    use xml_rs::reader::{EventReader, XmlEvent};

    pub fn xml_rs_parser(path: &Path) -> usize {
        let mut counter = 0;
        let file = File::open(path).unwrap();
        let file = BufReader::new(file);

        let parser = EventReader::new(file);
        for e in parser {
            match e {
                Ok(XmlEvent::StartElement {
                    name, attributes, ..
                }) => {
                    counter += name.local_name.len();
                    for attr in attributes {
                        counter += attr.value.len();
                    }
                }
                _ => (),
            }
        }
        counter
    }
}
use xml_rs_bench::xml_rs_parser;

bench!("tiny.xml", tiny_xml_rs, xml_rs_parser);
bench!("medium.xml", medium_xml_rs, xml_rs_parser);
bench!("large.xml", large_xml_rs, xml_rs_parser);
criterion_group!(xml_rs, tiny_xml_rs, medium_xml_rs, large_xml_rs);

criterion_main!(tiny, medium, large, utf_16, quick_xml, xml5ever, rustyxml, xml_rs);
