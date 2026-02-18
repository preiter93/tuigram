# Tuigram

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
