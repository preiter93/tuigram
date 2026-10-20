<h1 align="center">Tuigram</h1>

<p align="center">
  <a href="https://crates.io/crates/tuigram"><img src="https://img.shields.io/crates/v/tuigram?logo=rust&style=flat-square&logoColor=E05D44&color=E05D44" alt="Crate IO"></a>
  <img src="https://img.shields.io/crates/d/tuigram?style=flat-square" alt="Crates.io Downloads">
  <a href="https://github.com/preiter93/tuigram/actions/workflows/ci.yml"><img src="https://github.com/preiter93/tuigram/actions/workflows/ci.yml/badge.svg" alt="Continuous Integration"></a>
  <a href="https://deps.rs/repo/github/preiter93/tuigram"><img src="https://deps.rs/repo/github/preiter93/tuigram/status.svg?style=flat-square" alt="Deps Status"></a>
  <a href="./LICENSE"><img src="https://img.shields.io/crates/l/tuigram?style=flat-square&color=09bd66" alt="License"></a>
</p>

<p align="center">
  <img src="demo/demo.gif" alt="Demo">
</p>

<p align="center"><em>A TUI sequence diagram editor.</em></p>

## Installation

<details>
<summary><b>Homebrew</b></summary>

```
brew install preiter93/tuigram/tuigram
```
</details>

<details>
<summary><b>Cargo</b></summary>

```
cargo install tuigram
```
</details>

<details>
<summary><b>Nix</b></summary>

```
nix run github:preiter93/tuigram
```
</details>

<details>
<summary><b>Pre-built binaries</b></summary>

Download from [GitHub Releases](https://github.com/preiter93/tuigram/releases)
</details>

## Keybindings

| Key | Action |
|-----|--------|
| `p` | Add participant (with a box selected: add it as the box's last member) |
| `m/M` | Insert message after/before selected (inside its frame, if any) |
| `n/N` | Insert note after/before selected (inside its frame, if any) |
| `b/B` | Add box around participants / remove the selected or participant's box |
| `f/F` | Add/edit, remove frame (events stay) |
| `h/l` or `←/→` | Navigate left/right (participants, boxes) |
| `j/k` or `↓/↑` | Navigate down/up (messages/notes, frame headers) |
| `g/G` | Jump to first/last message/note (frame header if the first event starts a frame) |
| `H/L` or `Shift+←/→` | Move participant or whole box left/right, reverse message arrow |
| `a/A` | Cycle message arrow style forward/backward |
| `J/K` or `Shift+↓/↑` | Move message/note or whole frame up/down* |
| `Enter` | Edit selected (box: color and label) |
| `r` | Rename selected (participant: display name) |
| `t` | Toggle participant/actor |
| `#` | Toggle autonumber (numbers messages 1., 2., …) |
| `D` | Delete selected (a selected frame or box is removed, its events/participants stay) |
| `C` | Clear diagram |
| `u` / `U` or `Ctrl+r` | Undo / redo (up to 100 steps) |
| `S` | Save (path prompt prefilled with the opened file, else `diagram.mmd`; Enter saves) |
| `y` | Copy Mermaid to clipboard |
| `?` | Help (any key closes it) |
| `Ctrl+c` | Quit (asks if there are unsaved changes) |

> \* `Shift+↑/↓` may not work in all terminals, some intercept these for scrollback. Use `J`/`K` instead. At a frame edge `J`/`K` moves the event out of (or into) the frame instead of swapping; moving the only event out removes the frame.

Wide diagrams scroll sideways to follow the selection; `‹` and `›` on the border mark hidden columns.

## Mermaid

### Open / Save

```
tuigram diagram.mmd
```

`--import diagram.mmd` still works. A missing file starts an empty diagram and is created on save. Press `S` to save: the path prompt is prefilled with the opened file (default `diagram.mmd`), so `S` then `Enter` saves in place. Type a different path to save elsewhere (`~` is expanded, missing directories are not created, an existing file is overwritten without asking); it becomes the prefilled path from then on. A `[+]` marker in the status bar shows unsaved changes. Saving rewrites the file in canonical format, so `%%` comments, blank lines and custom formatting are dropped.

Blank lines and `%%` comments are ignored; unsupported syntax is reported with its line number.

Participant aliases (`participant A as Alice`) and `actor` declarations are supported.

Single-section frames `loop`, `opt`, `break` and `critical` (with an optional label, closed by `end`) are supported. In the editor, `f` marks the first event, extends the frame with `j`/`k`, and `Enter` picks the kind and label. Nested frames, frames inside boxes and boxes inside frames, and multi-section blocks (`alt`, `par`, `rect`) are rejected with a line number. With a frame selected (its header is a stop between events) `J`/`K` move the whole frame past one event or swap it with an adjacent frame, `Enter` edits kind and label, `D` removes the frame but keeps its events, `m`/`n` insert inside it as the first event and `M`/`N` insert above it.

A bare `autonumber` line before the first message or note numbers the messages (notes are not numbered). Options such as `autonumber 10 5` or `autonumber off` are rejected. Toggle it in the editor with `#`.

## Clipboard

`y` copies the diagram as Mermaid text using the OSC 52 terminal escape sequence. The terminal gives no confirmation, so "Copied N lines" only means the sequence was sent. Terminal.app ignores OSC 52, tmux needs `set -g set-clipboard on`, and some terminals limit the size or ask before accepting it.
