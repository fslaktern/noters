# Noters

## Features

- CRUD
- List notes
- Inline note references
- 100% safe Rust = no memory errors
- Privacy thanks to separation of ownership

## Available adapters

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
- Description: Blazingly fast, memory-safe, CRUD-compliant note taking app
- Difficulty: medium
- Category: pwn

## Vulnerability

The vulnerability exists on line 182 in `main.rs`. Here, instead of checking the content of `partial_note.id` for any links to the note we're trying to delete, it reads its own content to see if there are any references to itself.

### Patch

```diff
-let content = self.repo.read(id)?.content;
+let content = self.repo.read(partial_note.id)?.content;
```

### Exploit

To exploit this, we create two notes owned by us. Order doesn't matter. The first one can contain any content you like, but the second one should reference the first, like so:

```text
Reference to first note:
[[0]]
```

```md
 id | owner     | name 
----+-----------+----------
 0  | fslaktern | abc
 1  | fslaktern | ref-to-0 
```

Now, since the backlink-checking in `fn delete_note()` is broken, we can delete the first note, leaving a reference to the first note untouched. This is a high-level use-after-free vulnerability caused by a small typing error.

```md
 id | owner     | name 
----+-----------+---------
 1  | fslaktern | ref-to-0 
```

We continue by creating a note containing the flag. It will automatically place itself in the first available ID, which in our case is ID #0. This is the note ID that our `ref-to-0` note references.

```md
 id | owner                | name 
----+----------------------+---------
 0  | Norske Nøkkelsnikere | flag 
 1  | fslaktern            | ref-to-0 
```

To end it all, we just read the contents of note #1 and the reference to note #0 will unfold right before our eyes.

```
Please choose an option:
(1) Create note
(2) Read note
(3) Update note
(4) Delete note
(5) List notes
(6) Add note with flag

Choose option:
> 2

Read note:
id:
> 1

-------------------------------
#1: ref-to-0

Reference to first note:
>>> #0 flag
>
> NNSCTF{flag}
-------------------------------
```
