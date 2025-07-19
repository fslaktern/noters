# Noters

## Features

- CRUD
- List notes
- Cross platform
- Inline note references
- 100% safe Rust = no memory errors
- Privacy thanks to separation of ownership

## Available backends

- Filesystem
- SQLite

## Usage

The security that the `--user` flag provides is nullified if the user has access to restart the program as another user (or if they have access to read the contents of the backend directly).

```sh
noters --user "$USER" sqlite --path "notes.db"
noters --user "$USER" filesystem --path "./notes"
```

## Challenge

- Name: Noters
- Description: Blazingly fast, 100% memory-safe, CRUD-compliant, cross platform and private note taking app
- Difficulty: medium
- Category: pwn
- Program init:

    ```sh
    noters --user "ctf" --max-name-size 8 --max-content-size 128 --max-note-count 8 sqlite --path "notes.db"
    ```
