# Tuigram

[![Crate IO](https://img.shields.io/crates/v/tuigram?logo=rust&style=flat-square&logoColor=E05D44&color=E05D44)](https://crates.io/crates/tuigram) ![Crates.io Downloads](https://img.shields.io/crates/d/tuigram?style=flat-square) [![Continuous Integration](https://github.com/preiter93/tuigram/actions/workflows/ci.yml/badge.svg)](https://github.com/preiter93/tuigram/actions/workflows/ci.yml) [![Deps Status](https://deps.rs/repo/github/preiter93/tuigram/status.svg?style=flat-square)](https://deps.rs/repo/github/preiter93/tuigram) [![License](https://img.shields.io/crates/l/tuigram?style=flat-square&color=09bd66)](./LICENSE)

A TUI sequence diagram editor.

![Demo](demo/demo.gif)

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
| `p` | Add participant |
| `e` | Add event |
| `j/k/Tab` | Navigate next/previous |
| `h/l/S-Tab` | Navigate next/previous |
| `H/L` | Move participant left/right, reverse event arrow |
| `J/K` | Move event up/down |
| `d` | Delete selected |
| `M` | Export to Mermaid |
| `C` | Clear diagram |
| `?` | Help |
| `Ctrl+c` | Quit |

## Mermaid

### Export

Press `M` to export the diagram to `diagram.mmd`.

### Import

```
tuigram --import diagram.mmd
```
