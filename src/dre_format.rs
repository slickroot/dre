use crate::state;
use quick_xml::events::Event;
use quick_xml::se::Serializer;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename = "dre", deny_unknown_fields)]
pub(crate) struct FileDoc {
    #[serde(rename = "box", default)]
    pub(crate) boxes: Vec<FileBox>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileBox {
    #[serde(rename = "@label")]
    pub(crate) label: String,
    #[serde(rename = "@colour", default, skip_serializing_if = "Option::is_none")]
    pub(crate) colour: Option<u8>,
    #[serde(rename = "@fill", default, skip_serializing_if = "Option::is_none")]
    pub(crate) fill: Option<u8>,
    #[serde(rename = "@rounded", default, skip_serializing_if = "is_false")]
    pub(crate) rounded: bool,
    #[serde(rename = "box", default)]
    pub(crate) children: Vec<FileBox>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

pub(crate) fn write(doc: &FileDoc) -> String {
    let mut text = String::new();
    let mut serializer = Serializer::new(&mut text);
    serializer.indent(' ', 2);
    doc.serialize(serializer).expect("a diagram always serialises");
    text.push('\n');
    text
}

pub(crate) fn only_a_dre_root(text: &str) -> bool {
    let mut reader = Reader::from_str(text);
    loop {
        match reader.read_event() {
            Ok(Event::Decl(_) | Event::Comment(_)) => {}
            Ok(Event::Text(text)) => {
                if !text.as_ref().chars().all(char::is_whitespace) {
                    return false;
                }
            }
            Ok(Event::Start(tag) | Event::Empty(tag)) => return tag.name().as_ref() == "dre",
            _ => return false,
        }
    }
}

pub(crate) fn in_palette(doc: &FileDoc) -> bool {
    fn box_in_palette(file_box: &FileBox) -> bool {
        let colour_ok = file_box.colour.is_none_or(|i| i < state::PALETTE_SIZE);
        let fill_ok = file_box.fill.is_none_or(|i| i < state::PALETTE_SIZE);
        colour_ok && fill_ok && file_box.children.iter().all(box_in_palette)
    }
    doc.boxes.iter().all(box_in_palette)
}

pub(crate) fn read(text: &str) -> Option<FileDoc> {
    let doc: FileDoc = quick_xml::de::from_str(text).ok()?;
    if only_a_dre_root(text) && in_palette(&doc) {
        Some(doc)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(label: &str) -> FileBox {
        FileBox { label: label.to_string(), colour: None, fill: None, rounded: false, children: vec![] }
    }

    fn with_children(file_box: FileBox, children: Vec<FileBox>) -> FileBox {
        FileBox { children, ..file_box }
    }

    fn spec_example() -> FileDoc {
        let backup = plain("Backup");
        let replica = with_children(plain("Replica"), vec![backup]);
        let postgres = with_children(plain("Postgres"), vec![replica]);
        let orders = with_children(FileBox { fill: Some(1), ..plain("Orders") }, vec![postgres]);
        let gateway = FileBox {
            colour: Some(2),
            rounded: true,
            ..with_children(plain("API gateway"), vec![plain("Auth"), orders])
        };
        FileDoc { boxes: vec![gateway, plain("Billing")] }
    }

    const SPEC_EXAMPLE_TEXT: &str = "\
<dre>
  <box label=\"API gateway\" colour=\"2\" rounded=\"true\">
    <box label=\"Auth\"/>
    <box label=\"Orders\" fill=\"1\">
      <box label=\"Postgres\">
        <box label=\"Replica\">
          <box label=\"Backup\"/>
        </box>
      </box>
    </box>
  </box>
  <box label=\"Billing\"/>
</dre>
";

    #[test]
    fn writing_the_spec_example_produces_the_indented_xml() {
        assert_eq!(write(&spec_example()), SPEC_EXAMPLE_TEXT);
    }

    #[test]
    fn writing_a_single_plain_box_leaves_out_every_default_setting() {
        let doc = FileDoc { boxes: vec![plain("Auth")] };
        assert_eq!(write(&doc), "<dre>\n  <box label=\"Auth\"/>\n</dre>\n");
    }

    #[test]
    fn writing_an_empty_diagram_produces_an_empty_dre_element() {
        assert_eq!(write(&FileDoc::default()), "<dre/>\n");
    }

    #[test]
    fn writing_a_label_with_quotes_angle_brackets_and_ampersands_escapes_them() {
        let doc = FileDoc { boxes: vec![plain("say \"hi\" <to> A&B")] };
        let text = write(&doc);
        assert!(text.contains("label=\"say &quot;hi&quot; &lt;to&gt; A&amp;B\""), "{text}");
    }

    #[test]
    fn reading_what_was_written_gives_back_the_spec_example() {
        let doc = spec_example();
        assert_eq!(read(&write(&doc)), Some(doc));
    }

    #[test]
    fn reading_what_was_written_gives_back_a_single_plain_box() {
        let doc = FileDoc { boxes: vec![plain("Auth")] };
        assert_eq!(read(&write(&doc)), Some(doc));
    }

    #[test]
    fn reading_what_was_written_gives_back_an_empty_diagram() {
        assert_eq!(read(&write(&FileDoc::default())), Some(FileDoc::default()));
    }

    #[test]
    fn reading_what_was_written_gives_back_escaped_labels() {
        let doc = FileDoc { boxes: vec![plain("say \"hi\" <to> A&B")] };
        assert_eq!(read(&write(&doc)), Some(doc));
    }

    #[test]
    fn reading_what_was_written_gives_back_deeply_nested_boxes() {
        let mut deepest = plain("level 20");
        for depth in (0..20).rev() {
            deepest = with_children(plain(&format!("level {depth}")), vec![deepest]);
        }
        let doc = FileDoc { boxes: vec![deepest] };
        assert_eq!(read(&write(&doc)), Some(doc));
    }

    #[test]
    fn reading_accepts_reformatted_xml() {
        let text = "<dre>\n\n    <box   rounded='true' colour='2'   label='API gateway' >\n<box label='Auth'></box>\n\t<box fill=\"1\" label=\"Orders\"><box label=\"Postgres\"><box label=\"Replica\"><box label=\"Backup\"></box></box></box></box>\n</box><box label='Billing'   /></dre>";
        assert_eq!(read(text), Some(spec_example()));
    }

    #[test]
    fn reading_a_zero_byte_file_gives_nothing() {
        assert_eq!(read(""), None);
    }

    #[test]
    fn reading_malformed_xml_gives_nothing() {
        assert_eq!(read("<dre><box label=\"Auth\"></dre>"), None);
    }

    #[test]
    fn reading_a_box_without_a_label_gives_nothing() {
        assert_eq!(read("<dre><box colour=\"2\"/></dre>"), None);
    }

    #[test]
    fn reading_a_non_numeric_colour_gives_nothing() {
        assert_eq!(read("<dre><box label=\"Auth\" colour=\"red\"/></dre>"), None);
    }

    #[test]
    fn reading_an_unknown_attribute_gives_nothing() {
        assert_eq!(read("<dre><box label=\"A\" shadow=\"true\"/></dre>"), None);
    }

    #[test]
    fn reading_an_unknown_element_gives_nothing() {
        assert_eq!(read("<dre><arrow/></dre>"), None);
    }

    #[test]
    fn reading_an_unknown_element_inside_a_box_gives_nothing() {
        assert_eq!(read("<dre><box label=\"A\"><arrow/></box></dre>"), None);
    }

    #[test]
    fn reading_text_content_in_the_root_gives_nothing() {
        assert_eq!(read("<dre>hi</dre>"), None);
    }

    #[test]
    fn reading_text_content_in_a_box_gives_nothing() {
        assert_eq!(read("<dre><box label=\"A\">hi</box></dre>"), None);
    }

    #[test]
    fn reading_a_different_root_element_gives_nothing() {
        assert_eq!(read("<plans><box label=\"A\"/></plans>"), None);
    }

    #[test]
    fn reading_a_colour_outside_the_palette_gives_nothing() {
        assert_eq!(read("<dre><box label=\"A\" colour=\"5\"/></dre>"), None);
    }

    #[test]
    fn reading_a_fill_outside_the_palette_gives_nothing() {
        assert_eq!(read("<dre><box label=\"A\" fill=\"99\"/></dre>"), None);
    }

    #[test]
    fn reading_a_bad_colour_on_a_nested_box_gives_nothing() {
        assert_eq!(read("<dre><box label=\"A\"><box label=\"B\" colour=\"5\"/></box></dre>"), None);
    }

    #[test]
    fn reading_accepts_colour_and_fill_at_the_edges_of_the_palette() {
        let doc = FileDoc { boxes: vec![FileBox { colour: Some(4), fill: Some(0), ..plain("A") }] };
        assert_eq!(read("<dre><box label=\"A\" colour=\"4\" fill=\"0\"/></dre>"), Some(doc));
    }

    #[test]
    fn reading_accepts_an_xml_declaration_and_comments_and_whitespace_before_the_root() {
        let text = "<?xml version=\"1.0\"?>\n<!-- a comment -->\n\n<dre/>";
        assert_eq!(read(text), Some(FileDoc::default()));
    }
}
