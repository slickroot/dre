pub(crate) mod files;

use std::io;

use crate::state::State;
use crate::{dre_format, file_document, filesystem};
use files::Files;

#[cfg_attr(test, mockall::automock)]
pub(crate) trait StateStore {
    // automock needs the lifetime named: it cannot mock an elided reference inside Option.
    #[allow(clippy::needless_lifetimes)]
    fn load<'a>(&self, path: Option<&'a str>) -> io::Result<State>;
    fn save(&self, state: &State) -> io::Result<()>;
}

pub(crate) struct FileStateStore {
    files: Box<dyn Files>,
}

impl FileStateStore {
    pub(crate) fn new(files: Box<dyn Files>) -> Self {
        Self { files }
    }
}

impl StateStore for FileStateStore {
    fn load(&self, path: Option<&str>) -> io::Result<State> {
        let Some(path) = path else {
            return Ok(State::default());
        };
        let text = match self.files.read(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(State::new_file(path.to_string()))
            }
            result => result?,
        };
        let doc = dre_format::read(&text)
            .map(file_document::to_document)
            .ok_or_else(|| filesystem::invalid(path))?;
        Ok(State::open(doc, Some(path.to_string())))
    }

    fn save(&self, state: &State) -> io::Result<()> {
        match &state.save_to {
            Some(path) => self.files.write(
                path,
                &dre_format::write(&file_document::from_document(&state.doc)),
            ),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::files::MockFiles;
    use super::*;
    use crate::diagram::{self, Path};

    fn store_over(files: MockFiles) -> FileStateStore {
        FileStateStore::new(Box::new(files))
    }

    fn files_reading(path: &'static str, result: io::Result<String>) -> MockFiles {
        let mut files = MockFiles::new();
        files
            .expect_read()
            .withf(move |p| p == path)
            .times(1)
            .return_once(move |_| result);
        files
    }

    fn load_file(contents: &str) -> io::Result<State> {
        store_over(files_reading("a.dre", Ok(contents.to_string()))).load(Some("a.dre"))
    }

    #[test]
    fn no_argument_starts_from_an_empty_diagram() {
        let mut files = MockFiles::new();
        files.expect_read().never();
        let state = store_over(files).load(None).unwrap();
        assert!(state.doc.boxes.is_empty());
        assert_eq!(state.doc.selected, None);
        assert_eq!(state.save_to, None);
    }

    #[test]
    fn a_valid_file_loads_with_the_first_box_selected() {
        let text = dre_format::write(&dre_format::FileDoc {
            boxes: vec![dre_format::FileBox {
                label: "API".to_string(),
                colour: None,
                fill: None,
                rounded: false,
                children: vec![],
            }],
        });
        let state = load_file(&text).unwrap();
        assert_eq!(state.doc.boxes.len(), 1);
        assert_eq!(state.doc.boxes[0].label, "API");
        assert_eq!(
            state.doc.selected,
            Some(Path {
                ancestors: vec![],
                index: 0
            })
        );
    }

    #[test]
    fn a_valid_file_loads_saving_back_to_the_given_path() {
        assert_eq!(
            load_file("<dre/>").unwrap().save_to,
            Some("a.dre".to_string())
        );
    }

    #[test]
    fn a_file_with_no_boxes_loads_an_empty_canvas_with_nothing_selected() {
        let state = load_file("<dre/>").unwrap();
        assert!(state.doc.boxes.is_empty());
        assert_eq!(state.doc.selected, None);
    }

    #[test]
    fn a_zero_byte_file_is_invalid_data() {
        assert_eq!(
            load_file("").err().map(|e| e.kind()),
            Some(io::ErrorKind::InvalidData)
        );
    }

    #[test]
    fn a_malformed_file_is_invalid_data() {
        assert_eq!(
            load_file("<dre><box").err().map(|e| e.kind()),
            Some(io::ErrorKind::InvalidData)
        );
    }

    #[test]
    fn an_invalid_file_has_the_filesystem_invalid_error_message() {
        let err = load_file("<dre><box label=\"A\" colour=\"99\"/></dre>")
            .err()
            .unwrap();
        assert_eq!(err.to_string(), filesystem::invalid("a.dre").to_string());
    }

    #[test]
    fn a_valid_file_is_not_a_new_file() {
        assert!(!load_file("<dre/>").unwrap().new_file);
    }

    #[test]
    fn a_missing_file_loads_an_empty_canvas_saved_to_that_path() {
        let files = files_reading("missing.dre", Err(io::Error::from(io::ErrorKind::NotFound)));
        let state = store_over(files).load(Some("missing.dre")).unwrap();
        assert!(state.doc.boxes.is_empty());
        assert_eq!(state.doc.selected, None);
        assert_eq!(state.save_to, Some("missing.dre".to_string()));
        assert!(state.new_file);
    }

    #[test]
    fn a_read_failure_other_than_not_found_propagates() {
        let files = files_reading(
            "a.dre",
            Err(io::Error::from(io::ErrorKind::PermissionDenied)),
        );
        assert_eq!(
            store_over(files)
                .load(Some("a.dre"))
                .err()
                .map(|e| e.kind()),
            Some(io::ErrorKind::PermissionDenied)
        );
    }

    fn state_with_one_box_saving_to(path: Option<&str>) -> State {
        State::open(
            diagram::Document {
                boxes: vec![diagram::node("API")],
                selected: None,
            },
            path.map(str::to_string),
        )
    }

    #[test]
    fn saving_writes_the_document_to_where_the_state_saves_to() {
        let state = state_with_one_box_saving_to(Some("out.dre"));
        let expected = dre_format::write(&file_document::from_document(&state.doc));
        let mut files = MockFiles::new();
        files
            .expect_write()
            .withf(move |path, contents| path == "out.dre" && contents == expected)
            .times(1)
            .returning(|_, _| Ok(()));
        store_over(files).save(&state).unwrap();
    }

    #[test]
    fn saving_with_nowhere_to_save_writes_nothing() {
        let mut files = MockFiles::new();
        files.expect_write().never();
        store_over(files)
            .save(&state_with_one_box_saving_to(None))
            .unwrap();
    }
}
