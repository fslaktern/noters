use crate::{backends::NoteRepository, Note, PartialNote};
use nnsctf_pwn_1::Result;

#[derive(Debug)]
pub struct FilesystemBackend {}

impl FilesystemBackend {
    pub fn new(path: String) -> Self {
        dbg!(&path);
        Self {}
    }
}

impl NoteRepository for FilesystemBackend {
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
        Ok(vec![
            PartialNote {
                id: 0,
                name: "Hello world".to_string(),
                owner: "ctf".to_string(),
            },
            PartialNote {
                id: 1,
                name: "Flag!".to_string(),
                owner: "".to_string(),
            },
            PartialNote {
                id: 2,
                name: "Diary".to_string(),
                owner: "superman".to_string(),
            },
            PartialNote {
                id: 3,
                name: "Passwords".to_string(),
                owner: "fslaktern".to_string(),
            },
        ])
    }
}
