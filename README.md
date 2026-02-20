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
| `p` | Add participant |
| `m/M` | Insert message after/before selected |
| `n/N` | Insert note after/before selected |
| `h/l` or `←/→` | Navigate left/right (participants) |
| `j/k` or `↓/↑` | Navigate down/up (events) |
| `H/L` | Move participant left/right, reverse event arrow |
| `J/K` | Move event up/down |
| `Enter` | Edit selected |
| `r` | Rename selected |
| `d` | Delete selected |
| `E` | Export to Mermaid |
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
