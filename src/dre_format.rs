use quick_xml::se::Serializer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename = "dre")]
pub(crate) struct FileDoc {
    #[serde(rename = "box", default)]
    pub(crate) boxes: Vec<FileBox>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

pub(crate) fn read(text: &str) -> Option<FileDoc> {
    quick_xml::de::from_str(text).ok()
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
}
