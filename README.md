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
         tiny(4.8KB) medium(1.5MB)  large(25MB) medium(UTF-16, 3.0MB)
easy_xml:  67.017us     28.347ms     339.31ms         29.729ms
minidom:   96.403us     43.271ms     630.24ms
roxmltree: 49.020us     16.304ms     332.86ms
xmltree:   3964.2us     1228.5ms    21128.0ms
```

You can see the result of benchmarks [here](https://github.com/BlueGreenMagick/easy-xml/actions/runs/1291967402).