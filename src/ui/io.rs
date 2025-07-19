use super::{MenuError, NoteError, PartialNote, Result};
use crate::app::NoteService;
use crate::setup::{arguments, logging};
use crate::ui::cli;

use log::{error, info, trace, warn};
use std::fmt;

/// Abstraction for input/output
pub trait IO {
    /// Read a trimmed line of input ending at newline
    fn get_input(&self) -> Result<String>;
    /// Read multiple lines until a trimmed line equals `stop_at`
    fn get_input_until(&self, stop_at: &str) -> Result<String>;
    /// Display a list of selectable options
    fn show_menu(&self, options: &[impl std::fmt::Display]);
    /// Display a bolded title
    fn show_title(&self, title: &str);
    /// Render a table of partial notes
    fn show_notes_list(&self, table: Vec<PartialNote>);
    /// Print a plain text message
    fn show_text(&self, msg: &str);
}

/// CRUD and listing actions available in the menu
#[derive(Debug, Clone, Copy)]
pub enum MenuOption {
    Create = 1,
    Read = 2,
    Update = 3,
    Delete = 4,
    List = 5,
    AddFlag = 6,
}

/// All menu options in display order
pub const ALL_MENU_OPTIONS: [MenuOption; 6] = [
    MenuOption::Create,
    MenuOption::Read,
    MenuOption::Update,
    MenuOption::Delete,
    MenuOption::List,
    MenuOption::AddFlag,
];

/// Convert a numeric choice into a `MenuOption`
///
/// # Errors
///
/// Returns `Err(())` if the value does not map to a valid variant
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

/// Show the option number and label, e.g. `(1) Create note`
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

/// Dispatch chosen `MenuOption` to its handler
///
/// # Parameters
///
/// - `io`: I/O implementation
/// - `service`: Note service backend
/// - `option`: Selected menu option
#[must_use]
fn handle_menu_option(io: &impl IO, service: &NoteService, option: MenuOption) {
    match option {
        MenuOption::Create => handle_create(io, service),
        MenuOption::Read => handle_read(io, service),
        MenuOption::Update => handle_update(io, service),
        MenuOption::Delete => handle_delete(io, service),
        MenuOption::List => handle_list(io, service),
        MenuOption::AddFlag => handle_add_flag(service),
    }
}

/// Initialize logging, parse args, and enters the main menu loop
///
/// # Panics
///
/// Unreachable branch when an unexpected `NoteError` occurs
///
/// # Errors
///
/// Logs `MenuError` variants but never returns
#[must_use]
pub fn run() {
    logging::setup_log();
    let service = arguments::handle_args();
    let io = cli::Cli;

    loop {
        io.show_menu(&ALL_MENU_OPTIONS);
        match get_menu_input(&io) {
            Ok(opt) => handle_menu_option(&io, &service, opt),
            Err(NoteError::Menu(e)) => error!("{e}"),
            Err(_) => unreachable!(),
        }
    }
}

/// Try parsing input as `MenuOption` or return an error
///
/// # Parameters
///
/// - `io`: I/O implementation
///
/// # Returns
///
/// The chosen `MenuOption` on success
///
/// # Errors
///
/// Returns `NoteError::Menu(MenuError::ParseError)` if input is not an integer
/// Returns `NoteError::Menu(MenuError::InvalidOption)` if integer is out of range
fn get_menu_input(io: &impl IO) -> Result<MenuOption> {
    let input = io.get_input()?;

    match input.parse::<u8>() {
        Ok(n) => match MenuOption::try_from(n) {
            Ok(option) => Ok(option),
            Err(_) => Err(NoteError::Menu(MenuError::InvalidOption(n))),
        },
        Err(_) => Err(NoteError::Menu(MenuError::ParseError(input))),
    }
}

/// Prompt for note creation and invoke service
///
/// # Parameters
///
/// - `io`: I/O implementation
/// - `service`: Note service backend
///
/// # Panics
///
/// If reading name or content fails unexpectedly
fn handle_create(io: &impl IO, service: &NoteService) {
    io.show_title("Create note");

    let name: String = loop {
        io.show_text("Name:");
        let input = io.get_input().expect("Failed getting note name");
        match NoteService::validate_name(&input, service.max_name_size) {
            Ok(()) => {
                trace!("Got valid name: {input}");
                break input;
            }
            Err(e) => error!("{e}"),
        }
    };

    let content: String = loop {
        // Stop when getting a "." alone on a line
        io.show_text("Content (end with '.' on last line):");
        let input = io
            .get_input_until(".")
            .expect("Failed getting note content");
        match NoteService::validate_content(&input, service.max_content_size) {
            Ok(()) => {
                trace!("Got valid content: {}", &input);
                break input;
            }
            Err(e) => error!("Got invalid content: {}", e),
        }
    };

    match service.create_note(name, content) {
        Ok(id) => info!("Note saved with ID: {}", id),
        Err(e) => error!("{e}"),
    };
}

/// Prompt for a note ID, fetch and display the note
///
/// # Parameters
///
/// - `io`: I/O implementation
/// - `service`: Note service backend
///
/// # Panics
///
/// If reading the ID fails unexpectedly
fn handle_read(io: &impl IO, service: &NoteService) {
    io.show_title("Read note");

    let id: u16 = loop {
        io.show_text("ID:");
        let input = io.get_input().expect("Failed getting note ID");
        match input.parse::<u16>() {
            Ok(id) => {
                trace!("Got valid ID: {}", id);
                break id;
            }
            Err(e) => error!("Got invalid ID: {}", e),
        }
    };

    match service.read_note(id) {
        Ok(note) => {
            io.show_text(&"-".repeat(20));
            io.show_text(&format!("#{}: {}\n", note.id, note.name));
            io.show_text(&note.content);
            io.show_text(&"-".repeat(20));
        }
        Err(e) => error!("{e}"),
    }
}

/// Prompt for note ID, updated fields, and apply update
///
/// # Parameters
///
/// - `io`: I/O implementation
/// - `service`: Note service backend
///
/// # Panics
///
/// If reading name or content fails unexpectedly
fn handle_update(io: &impl IO, service: &NoteService) {
    io.show_title("Update note");

    let mut note = loop {
        io.show_text("ID:");
        let input = io.get_input().expect("Failed getting note ID");
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
        io.show_text("Name:");
        let input = io.get_input().expect("Failed getting note name");
        match NoteService::validate_name(&input, service.max_name_size) {
            Ok(()) => {
                trace!("Got valid name: {}", &input);
                break input;
            }
            Err(e) => error!("Got invalid name: {}", e),
        }
    };

    let content: String = loop {
        // Stop when getting a "." alone on a line
        io.show_text("Content (end with '.' on last line):");
        let input = io
            .get_input_until(".")
            .expect("Failed getting note content");
        match NoteService::validate_content(&input, service.max_content_size) {
            Ok(()) => {
                trace!("Got valid content: {}", &input);
                break input;
            }
            Err(e) => error!("Got invalid content: {}", e),
        }
    };

    note.name = name;
    note.content = content;

    match service.update_note(note) {
        Ok(()) => info!("Successfully updated note"),
        Err(e) => error!("{e}"),
    };
}

/// Prompt for note ID, confirm deletion, and delete
///
/// # Parameters
///
/// - `io`: I/O implementation
/// - `service`: Note service backend
///
/// # Panics
///
/// If reading confirmation fails unexpectedly
fn handle_delete(io: &impl IO, service: &NoteService) {
    io.show_title("Delete note");

    let id: u16 = loop {
        io.show_text("ID:");
        let input = io.get_input().expect("Failed getting note ID");
        match input.parse::<u16>() {
            Ok(id) => break id,
            Err(e) => error!("{e}"),
        }
    };

    loop {
        io.show_text("Are you absolutely sure? (y/n):");
        let input = io.get_input().expect("Failed getting delete confirmation");
        match input.to_lowercase().as_str() {
            "y" | "ye" | "yes" | "ya" | "yuh" | "yarr" | "fuck yeah" => break,
            "n" | "nu uh" | "no" | "nah" | "hell naw" | "get yo bitchass outta here" => {
                info!("Exiting. Not deleting note with ID: {}", id);
                return;
            }
            _ => warn!("Invalid input. Please enter 'y' or 'n'"),
        }
    }

    match service.delete_note(id) {
        Ok(()) => info!("Successfully deleted note with ID: {}", id),
        Err(e) => error!("{e}"),
    };
}

/// Fetch all notes and display in a table
///
/// # Parameters
///
/// - `io`: Console I/O implementation
/// - `service`: Note service backend
fn handle_list(io: &impl IO, service: &NoteService) {
    let partial_notes: Vec<PartialNote> = match service.list_notes() {
        Ok(n) => n,
        Err(e) => {
            error!("{e}");
            return;
        }
    };
    io.show_notes_list(partial_notes);
}

/// Create a note containing the flag via service
///
/// # Parameters
///
/// - `service`: Note service backend
fn handle_add_flag(service: &NoteService) {
    match service.create_flag_note() {
        Ok(id) => info!("Successfully added note containing flag, with ID: {}", id),
        Err(e) => error!("Failed adding note containing flag: {}", e),
    }
}
