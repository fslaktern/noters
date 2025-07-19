use crate::{Note, NoteBackend, NoteError, NoteValidationError, PartialNote, Result};
use log::trace;
use std::collections::HashSet;

pub struct NoteService {
    pub repo: Box<dyn NoteBackend>,
    pub user: String,
    pub max_name_size: u8,
    pub max_content_size: u16,
    pub max_note_count: u16,
}

impl NoteService {
    pub fn new(
        repo: Box<dyn NoteBackend>,
        user: String,
        max_name_size: u8,
        max_content_size: u16,
        max_note_count: u16,
    ) -> Self {
        Self {
            repo,
            user,
            max_name_size,
            max_content_size,
            max_note_count,
        }
    }

    // Retrieve list of all notes (without content)
    pub fn list_notes(&self) -> Result<Vec<PartialNote>> {
        self.repo.list()
    }

    // Create a new note after validation and reference checks
    pub fn create_note(&self, name: String, content: String) -> Result<u16> {
        Self::validate_name(&name, self.max_name_size)?;
        Self::validate_content(&content, self.max_content_size)?;

        // Make sure not too many notes are created
        let notes = self.repo.list()?;
        if notes.len() > self.max_note_count as usize {
            return Err(NoteValidationError::NoteCountRateLimit {
                max: self.max_note_count,
            }
            .into());
        }

        // Find next free ID
        let used_ids: HashSet<u16> = notes.into_iter().map(|note| note.id).collect();
        let available_id = (0..self.max_note_count)
            .find(|id| !used_ids.contains(id))
            .expect("We just checked count limit; there must be an available id, or the sun caused a bit flip");

        // Make sure all referenced notes actually exist
        // Search for references in this format: " [[1]] " where 1 is the id of the referenced note
        for id in Self::get_references(&content) {
            if !used_ids.contains(&id) {
                return Err(NoteValidationError::ReferenceNotFound(id).into());
            }

            let partial_note: PartialNote = Self::get_partial_note(self, id)?;
            if partial_note.owner != self.user {
                return Err(NoteValidationError::PermissionDenied(id).into());
            }
        }

        let note = Note {
            id: available_id,
            // The creator is the owner
            owner: self.user.clone(),
            name,
            content,
        };

        self.repo.create(note)
    }

    // Read a full note and expand references (e.g. [[1]] expands to the name and content of note #1)
    pub fn read_note(&self, id: u16) -> Result<Note> {
        let mut note = self.repo.read(id)?;

        // Only allow owner read access
        if self.user != note.owner {
            return Err(NoteValidationError::PermissionDenied(id).into());
        }

        // Mapping references to note contents: [[1]] -> "Some content"
        let placeholders = Self::get_references(&note.content)
            .into_iter()
            .map(|rid| match self.repo.read(rid) {
                Ok(ref_note) => {
                    let placeholder = format!("[[{rid}]]");
                    let expansion = format!(
                        ">>> #{} {}\n>\n> {}",
                        ref_note.id,
                        ref_note.name,
                        ref_note.content.replace('\n', "\n> ")
                    );
                    Ok((placeholder, expansion))
                }
                Err(_) => Err(NoteValidationError::ReferenceNotFound(rid).into()),
            })
            .collect::<Result<Vec<(String, String)>>>()?;

        // Expanding references: [[1]] -> Note #1's content
        let expanded = placeholders
            .into_iter()
            .fold(note.content, |txt, (ph, exp)| txt.replace(&ph, &exp));

        note.content = expanded;
        Ok(note)
    }

    // Update existing note after validation
    // ID stays the same, but name and content is replaced
    pub fn update_note(&self, note: Note) -> Result<()> {
        Self::validate_name(&note.name, self.max_name_size)?;
        Self::validate_content(&note.content, self.max_content_size)?;

        let notes = self.repo.list()?;
        let used_ids: HashSet<u16> = notes.into_iter().map(|note| note.id).collect();

        // Make sure all referenced notes actually exist
        // Search for references in this format: " [[1]] " where 1 is the id of the referenced note
        for id in Self::get_references(&note.content) {
            if !used_ids.contains(&id) {
                return Err(NoteValidationError::ReferenceNotFound(id).into());
            }

            // Make sure the user is allowed to read the referenced note
            let partial_note: PartialNote = Self::get_partial_note(self, id)?;
            if partial_note.owner != self.user {
                return Err(NoteValidationError::PermissionDenied(id).into());
            }
        }

        // Make sure the note we are updating actually exist
        if used_ids.contains(&note.id) {
            self.repo.update(note)
        } else {
            Err(NoteValidationError::NoteNotFound(note.id).into())
        }
    }

    // Delete note by ID
    pub fn delete_note(&self, id: u16) -> Result<()> {
        // Check if any other note references this note (expensive)
        // and do not stop at the first backlink, find all of them
        let mut backlinks: Vec<u16> = Vec::new();
        for partial_note in self.list_notes()? {
            // Do not prevent deletion if note refers to itself
            if partial_note.id == id {
                continue;
            }

            // Read content and find all references
            // Save ID to Vec if it contains a backlink
            // to the note we're trying to delete
            let content = self.repo.read(id)?.content;
            let references = NoteService::get_references(&content);
            if references.contains(&id) {
                backlinks.push(partial_note.id);
            }
        }

        let num_backlinks = backlinks.len();
        trace!("Found {} backlinks to note with ID: {}", num_backlinks, id);
        match num_backlinks {
            0 => self.repo.delete(id),
            _ => Err(NoteError::Validation(
                NoteValidationError::NoteIsReferenced(backlinks),
            )),
        }
    }

    pub fn create_flag_note(&self) -> Result<u16> {
        // Make sure not too many notes are created
        let notes = self.repo.list()?;
        if notes.len() > self.max_note_count as usize {
            return Err(NoteValidationError::NoteCountRateLimit {
                max: self.max_note_count,
            }
            .into());
        }

        // Find next free ID
        let used_ids: HashSet<u16> = notes.into_iter().map(|note| note.id).collect();
        let available_id = (0..self.max_note_count)
            .find(|id| !used_ids.contains(id))
            .expect("We just checked count limit; there must be an available id, or the sun caused a bit flip");

        // The creator is the owner
        let note = Note {
            id: available_id,
            owner: "Norske Nøkkelsnikere".to_string(),
            name: "flag".to_string(),
            content: "NNSCTF{testing}".to_string(),
        };

        self.repo.create(note)
    }

    // --- small helpers ---

    // Validate note name
    pub fn validate_name(name: &str, max: u8) -> Result<()> {
        if name.trim().is_empty() {
            Err(NoteValidationError::NameEmpty.into())
        } else if name.len() > max as usize {
            Err(NoteValidationError::NameTooLarge {
                max,
                got: name.len(),
            }
            .into())
        } else {
            Ok(())
        }
    }

    // Validate note content
    pub fn validate_content(content: &str, max: u16) -> Result<()> {
        if content.trim().is_empty() {
            Err(NoteValidationError::ContentEmpty.into())
        } else if content.len() > max as usize {
            Err(NoteValidationError::ContentTooLarge {
                max,
                got: content.len(),
            }
            .into())
        } else {
            Ok(())
        }
    }

    // Extract referenced note IDs from content
    fn get_references(s: &str) -> Vec<u16> {
        s.split_whitespace()
            .filter_map(|tok| {
                if tok.starts_with("[[") && tok.ends_with("]]") {
                    tok[2..tok.len() - 2].parse().ok()
                } else {
                    None
                }
            })
            .collect()
    }

    // Extract owner name from note
    fn get_partial_note(&self, id: u16) -> Result<PartialNote> {
        self.repo.read_partial(id)
    }
}
