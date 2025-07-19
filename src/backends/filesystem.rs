use super::{Note, NoteBackend, PartialNote, Result};

#[derive(Debug)]
pub struct FilesystemBackend {}

impl FilesystemBackend {
    /// Creates a new instance of the `FilesystemBackend`.
    ///
    /// # Arguments
    ///
    /// * `path` - A `String` representing the path to the notes storage (currently unused).
    ///
    /// # Returns
    ///
    /// A new `FilesystemBackend` instance.
    #[must_use]
    pub fn new(path: &str) -> Self {
        dbg!(&path);
        Self {}
    }
}

impl NoteBackend for FilesystemBackend {
    fn create(&self, note: Note) -> Result<u16> {
        dbg!(&note);
        Ok(0)
    }

    fn read(&self, id: u16) -> Result<Note> {
        dbg!(&id);
        Ok(Note {
            id: 0,
            name: "Hello world".to_string(),
            owner: "fslaktern".to_string(),
            content: "I am delighted to exist!".to_string(),
        })
    }

    fn read_partial(&self, id: u16) -> Result<PartialNote> {
        dbg!(&id);
        Ok(PartialNote {
            id: 0,
            name: "Hello world".to_string(),
            owner: "fslaktern".to_string(),
        })
    }

    fn update(&self, note: Note) -> Result<()> {
        dbg!(&note);
        Ok(())
    }

    fn delete(&self, id: u16) -> Result<()> {
        dbg!(&id);
        Ok(())
    }

    fn list(&self) -> Result<Vec<PartialNote>> {
        Ok(vec![])
    }
}
