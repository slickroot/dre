use std::io;

use crate::state::State;
use crate::{dre_format, file_document, filesystem};

pub(crate) trait Files {
    fn read(&self, path: &str) -> io::Result<String>;
    fn write(&self, path: &str, contents: &str) -> io::Result<()>;
}

pub(crate) trait StateStore {
    fn load(&self, path: Option<&str>) -> io::Result<State>;
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
    use super::*;
    use crate::diagram::{self, Path};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    #[derive(Clone, Default)]
    struct FakeFiles {
        contents: Rc<RefCell<HashMap<String, String>>>,
    }

    impl FakeFiles {
        fn with(path: &str, contents: &str) -> Self {
            let files = Self::default();
            files
                .contents
                .borrow_mut()
                .insert(path.to_string(), contents.to_string());
            files
        }

        fn contents_of(&self, path: &str) -> Option<String> {
            self.contents.borrow().get(path).cloned()
        }

        fn is_empty(&self) -> bool {
            self.contents.borrow().is_empty()
        }
    }

    impl Files for FakeFiles {
        fn read(&self, path: &str) -> io::Result<String> {
            self.contents
                .borrow()
                .get(path)
                .cloned()
                .ok_or_else(|| filesystem::missing(path))
        }

        fn write(&self, path: &str, contents: &str) -> io::Result<()> {
            self.contents
                .borrow_mut()
                .insert(path.to_string(), contents.to_string());
            Ok(())
        }
    }

    fn store_over(files: &FakeFiles) -> FileStateStore {
        FileStateStore::new(Box::new(files.clone()))
    }

    fn load_file(contents: &str) -> io::Result<State> {
        store_over(&FakeFiles::with("a.dre", contents)).load(Some("a.dre"))
    }

    #[test]
    fn no_argument_starts_from_an_empty_diagram() {
        let state = store_over(&FakeFiles::default()).load(None).unwrap();
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
        let state = store_over(&FakeFiles::default())
            .load(Some("missing.dre"))
            .unwrap();
        assert!(state.doc.boxes.is_empty());
        assert_eq!(state.doc.selected, None);
        assert_eq!(state.save_to, Some("missing.dre".to_string()));
        assert!(state.new_file);
    }

    #[test]
    fn a_read_failure_other_than_not_found_propagates() {
        struct DeniedFiles;
        impl Files for DeniedFiles {
            fn read(&self, _: &str) -> io::Result<String> {
                Err(io::Error::from(io::ErrorKind::PermissionDenied))
            }
            fn write(&self, _: &str, _: &str) -> io::Result<()> {
                Ok(())
            }
        }
        let result = FileStateStore::new(Box::new(DeniedFiles)).load(Some("a.dre"));
        assert_eq!(
            result.err().map(|e| e.kind()),
            Some(io::ErrorKind::PermissionDenied)
        );
    }

    fn state_with_one_box_saving_to(path: Option<&str>) -> State {
        let mut state = State::open(
            diagram::Document {
                boxes: vec![diagram::node("API")],
                selected: None,
            },
            path.map(str::to_string),
        );
        state.save_to = path.map(str::to_string);
        state
    }

    #[test]
    fn saving_writes_the_document_to_where_the_state_saves_to() {
        let files = FakeFiles::default();
        let state = state_with_one_box_saving_to(Some("out.dre"));
        store_over(&files).save(&state).unwrap();
        let expected = dre_format::write(&file_document::from_document(&state.doc));
        assert_eq!(files.contents_of("out.dre"), Some(expected));
    }

    #[test]
    fn saving_with_nowhere_to_save_writes_nothing() {
        let files = FakeFiles::default();
        store_over(&files)
            .save(&state_with_one_box_saving_to(None))
            .unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn a_saved_state_loads_back_with_the_same_boxes() {
        let files = FakeFiles::default();
        let store = store_over(&files);
        store
            .save(&state_with_one_box_saving_to(Some("out.dre")))
            .unwrap();
        let loaded = store.load(Some("out.dre")).unwrap();
        assert_eq!(loaded.doc.boxes.len(), 1);
        assert_eq!(loaded.doc.boxes[0].label, "API");
    }
}
