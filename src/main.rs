use crate::backends::NoteRepository;
use log::{error, info, trace, warn};
use nnsctf_pwn_1::{MenuError, NoteError, NoteValidationError, Result};
use std::collections::HashSet;
use std::convert::TryFrom;
use std::fmt;
use std::io::{self, Write};
use tabled::{settings::Style, Table, Tabled};

mod arguments;
mod backends;
mod logging;

#[derive(Tabled, Debug)]
pub struct Note {
    pub id: u16,
    pub owner: String,
    pub name: String,
    pub content: String,
}

// Partial note data. Displayed in lists and for shallow reads
#[derive(Tabled)]
pub struct PartialNote {
    pub id: u16,
    pub owner: String,
    pub name: String,
}

pub struct NoteService {
    repo: Box<dyn NoteRepository>,
    user: String,
    max_name_size: u8,
    max_content_size: u16,
    max_note_count: u16,
}

impl NoteService {
    pub fn new(
        repo: Box<dyn NoteRepository>,
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

        // The creator is the owner
        let note = Note {
            id: available_id,
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

// All UI menu options
#[derive(Debug, Clone, Copy)]
pub enum MenuOption {
    Create = 1,
    Read = 2,
    Update = 3,
    Delete = 4,
    List = 5,
    AddFlag = 6,
}

// Convert integer to MenuOption
impl TryFrom<u8> for MenuOption {
    type Error = ();

    fn try_from(n: u8) -> std::result::Result<Self, Self::Error> {
        match n {
            1 => Ok(MenuOption::Create),
            2 => Ok(MenuOption::Read),
            3 => Ok(MenuOption::Update),
            4 => Ok(MenuOption::Delete),
            5 => Ok(MenuOption::List),
            6 => Ok(MenuOption::AddFlag),
            _ => Err(()),
        }
    }
}

// Display a menu option with number and label
impl fmt::Display for MenuOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            MenuOption::Create => "Create note",
            MenuOption::Read => "Read note",
            MenuOption::Update => "Update note",
            MenuOption::Delete => "Delete note",
            MenuOption::List => "List notes",
            MenuOption::AddFlag => "Add note with flag",
        };
        write!(f, "({}) {}", *self as u8, label)
    }
}

// Array of all menu options. Used for iteration
const ALL_MENU_OPTIONS: [MenuOption; 6] = [
    MenuOption::Create,
    MenuOption::Read,
    MenuOption::Update,
    MenuOption::Delete,
    MenuOption::List,
    MenuOption::AddFlag,
];

// Display menu and options
fn show_menu() {
    println!("Please choose an option:");
    for option in &ALL_MENU_OPTIONS {
        println!("{option}");
    }
    println!()
}

// Prompt for menu option input and return parsed option
fn get_menu_input(prompt: &str) -> Result<MenuOption> {
    loop {
        let input = get_input(prompt)?;

        match input.parse::<u8>() {
            Ok(n) => match MenuOption::try_from(n) {
                Ok(option) => return Ok(option),
                Err(_) => {
                    error!("Value not in range 1-6: {}", MenuError::ParseError);
                    continue;
                }
            },
            Err(_) => {
                error!(
                    "Failed reading input as a number: {}",
                    MenuError::ParseError
                );
                continue;
            }
        }
    }
}

// Prompt for user input and trim result
fn get_input(prompt: &str) -> Result<String> {
    loop {
        print!("{prompt}");
        match io::stdout().flush() {
            Ok(()) => trace!("Flushed stdout"),
            Err(e) => warn!("Failed flushing stdout: {e}"),
        };

        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(_) => trace!("Got input: {}", &line),
            Err(e) => {
                error!("Failed reading input: {e}");
                return Err(NoteError::Menu(MenuError::StdinReadError(e)));
            }
        };
        println!();

        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
}

// Prompt for user input, read until [stop_at] and trim result
fn get_input_until(prompt: &str, stop_at: &str) -> Result<String> {
    print!("{prompt}");
    match io::stdout().flush() {
        Ok(()) => trace!("Flushed stdout"),
        Err(e) => warn!("Failed flushing stdout: {e}"),
    };

    let mut input = String::new();
    loop {
        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(_) => trace!("Got input: {}", &line),
            Err(e) => {
                error!("Failed reading input: {e}");
                return Err(NoteError::Menu(MenuError::StdinReadError(e)));
            }
        };

        if line.trim() == stop_at {
            input = input.trim().to_string();
            break;
        } else {
            input += &line;
        }
    }

    Ok(input)
}

// Handle "create note" menu option
fn handle_create(service: &NoteService) {
    println!("Create note:");

    let name: String = loop {
        let input = get_input("name:\n> ").expect("Failed getting note name");
        match NoteService::validate_name(&input, service.max_name_size) {
            Ok(()) => {
                trace!("Got valid name: {}", &input);
                break input;
            }
            Err(e) => error!("Got invalid name: {}", e),
        }
        println!();
    };

    let content: String = loop {
        // Stop when getting a "." alone on a line
        let input = get_input_until("content (end with '.' on last line):\n", ".")
            .expect("Failed getting note content");
        match NoteService::validate_content(&input, service.max_content_size) {
            Ok(()) => {
                trace!("Got valid content: {}", &input);
                break input;
            }
            Err(e) => error!("Got invalid content: {e}"),
        }
        println!();
    };

    match service.create_note(name, content) {
        Ok(id) => info!("Note saved with ID #{}", id),
        Err(NoteError::Repository(e)) => error!("Backend error: {e}"),
        Err(e) => error!("{e}"),
    };
}

// Handle "read note" menu option
fn handle_read(service: &NoteService) {
    println!("Read note:");

    let id: u16 = loop {
        let input = get_input("id:\n> ").expect("Failed getting note ID");
        match input.parse::<u16>() {
            Ok(id) => {
                trace!("Got valid ID: {}", id);
                break id;
            }
            Err(e) => error!("Got invalid ID: {e}"),
        }
        println!();
    };

    match service.read_note(id) {
        Ok(note) => {
            println!("-------------------------------");
            println!("#{}: {}\n", note.id, note.name);
            println!("{}", note.content);
            println!("-------------------------------");
        }
        Err(NoteError::Repository(e)) => error!("Backend error: {e}"),
        Err(e) => error!("{e}"),
    }
}

// Handle "update note" menu option
fn handle_update(service: &NoteService) {
    println!("Update note:");

    let mut note = loop {
        let input = get_input("id:\n> ").expect("Failed getting note ID");
        let id = match input.parse::<u16>() {
            Ok(id) => id,
            Err(e) => {
                error!("{e}");
                continue;
            }
        };
        match service.read_note(id) {
            Ok(note) => break note,
            Err(e) => error!("{e}"),
        };
    };

    let name: String = loop {
        let input = get_input("name:\n> ").expect("Failed getting note name");
        match NoteService::validate_name(&input, service.max_name_size) {
            Ok(()) => break input,
            Err(e) => error!("{e}"),
        }
    };

    let content: String = loop {
        let input = get_input_until("content (end with '.' on last line):\n", ".")
            .expect("Failed getting note content");
        match NoteService::validate_content(&input, service.max_content_size) {
            Ok(()) => break input,
            Err(e) => error!("{e}"),
        }
    };

    note.name = name.to_string();
    note.content = content.to_string();

    match service.update_note(note) {
        Ok(()) => info!("Successfully updated note"),
        Err(NoteError::Repository(e)) => error!("Backend error {e}"),
        Err(e) => error!("{e}"),
    };
}

// Handle "delete note" menu option with confirmation prompt
fn handle_delete(service: &NoteService) {
    println!("Delete note:");

    let id: u16 = loop {
        let input = get_input("id:\n> ").expect("Failed getting note ID");
        match input.parse::<u16>() {
            Ok(id) => break id,
            Err(e) => error!("{e}"),
        }
    };

    loop {
        let input = get_input("Are you absolutely sure? (y/n):\n> ")
            .expect("Failed getting delete confirmation");
        match input.to_lowercase().as_str() {
            "y" | "ye" | "yes" | "ya" | "yuh" | "yarr" | "fuck yeah" => break,
            "n" | "nu uh" | "no" | "nah" | "hell naw" | "get yo bitchass outta here" => {
                info!("Exiting. Not deleting note #{}", id);
                return;
            }
            _ => continue,
        }
    }

    match service.delete_note(id) {
        Ok(()) => info!("Successfully deleted note #{}", id),
        Err(NoteError::Repository(e)) => error!("Backend error {e}"),
        Err(e) => error!("{e}"),
    };
}

// Handle "list notes" menu option
fn handle_list(service: &NoteService) {
    let partial_notes: Vec<PartialNote> = match service.list_notes() {
        Ok(n) => n,
        Err(NoteError::Repository(e)) => {
            error!("Backend error {e}");
            return;
        }
        Err(e) => {
            error!("{e}");
            return;
        }
    };

    let mut table = Table::new(partial_notes);
    table.with(Style::psql());
    println!("{}", table);
}

// Handle "add note with flag" menu option
fn handle_add_flag(service: &NoteService) {
    match service.create_flag_note() {
        Ok(id) => info!("Successfully added note containing flag, with ID: {}", id),
        Err(e) => error!("Failed adding note containing flag: {}", e),
    }
}

// Route selected menu option to its handler function
fn handle_menu_option(service: &NoteService, option: MenuOption) {
    match option {
        MenuOption::Create => handle_create(service),
        MenuOption::Read => handle_read(service),
        MenuOption::Update => handle_update(service),
        MenuOption::Delete => handle_delete(service),
        MenuOption::List => handle_list(service),
        MenuOption::AddFlag => handle_add_flag(service),
    }
}

fn main() {
    logging::setup();

    let service: NoteService = arguments::handle_args();

    loop {
        show_menu();
        match get_menu_input("Choose option:\n> ") {
            Ok(option) => handle_menu_option(&service, option),
            Err(e) => match e {
                NoteError::Menu(MenuError::ParseError) => error!("Failed to parse your input"),
                NoteError::Menu(MenuError::StdinReadError(e)) => {
                    error!("Error when reading menu choice: {}", e)
                }
                other => error!("Unexpected menu error: {}", other),
            },
        };
        println!();
    }
}
