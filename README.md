# easy-xml

easy-xml is a rust library to read, modify, and write XML documents.

Features:
- Most encodings can be read, including UTF-16, ISO 8859-1, GBK and EUC-KR. (With the notable exception of UTF-32)
- You can have references to the parts of the tree, and still mutate the tree.
- Elements stores reference to its parent element, so traveling up the tree is fast.
- One of the fastest XML tree-like parser. See [performance](https://github.com/bluegreenmagick/easy-xml#performance) section.

## Example

```rust
use easy_xml::{Document, Element};

let XML = r#"<?xml version="1.0"?>
<package xmlns:dc="http://purl.org/dc/elements/1.1/">
    <metadata>
        <dc:title>easy-xml</dc:title>
        <dc:rights>MIT or Apache 2.0</dc:rights>
    </metadata>
</package>
"#;


let doc = Document::new();
doc.parse_str(XML);
let metadata = doc.root_element().unwrap().find(&doc, "metadata").unwrap();
let title = metadata.find(&doc, "title").unwrap();
title.set_attribute("xml:lang", "en");

// Add an element to metadata: <dc:creator id="author">Yoonchae Lee</dc:creator>
let author = Element::build(&mut doc, "dc:creator")
    .text_content("Yoonchae Lee")
    .attribute("id", "author")
    .push_to(metadata);

let new_xml = doc.write_str();
```

## Performance
### Parsing
```
// tiny.xml (4.8KB)
easy_xml: 67.017us
minidom: 96.403us
roxmltree: 49.020us
xmltree: 3964.2ms

// medium.xml (1.5MB)
easy_xml: 28.347ms
minidom: 43.271ms
roxmltree: 16.304ms
xmltree: 1228.5ms

// large.xml (25MB)
easy_xml: 339.31ms
minidom: 630.24ms
roxmltree: 332.86ms
xmltree: 21128.0ms

// medium_utf16.xml (3.0MB) (medium.xml in UTF-16)
easy_xml: 29.729ms
```

You can see the result of benchmarks [here](https://github.com/BlueGreenMagick/easy-xml/actions/runs/1291967402).