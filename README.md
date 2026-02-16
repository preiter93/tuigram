# Tuigram

A TUI sequence diagram editor.

![Demo](demo/demo.gif)

## Installation

### Homebrew

```
brew install preiter93/tuigram/tuigram
```

### Nix

```
nix run github:preiter93/tuigram
```

Or install it to your profile:

```
nix profile install github:preiter93/tuigram
```

### From crates.io

```
cargo install tuigram
```

### Pre-built binaries

Download the latest release from [GitHub Releases](https://github.com/preiter93/tuigram/releases).

## Keybindings

| Key | Action |
|-----|--------|
| `p` | Add participant |
| `e` | Add event |
| `h/l` | Navigate participants |
| `j/k` | Navigate events |
| `H/L` | Move participant left/right |
| `J/K` | Move event up/down |
| `d` | Delete selected |
| `Tab` | Cycle selection |
| `m` | Export to Mermaid |
| `?` | Help |
| `Ctrl+c` | Quit |

## Export to Mermaid

Press `m` to export the diagram to `diagram.mmd` in Mermaid format.

### Import from Mermaid

```
tuigram --import diagram.mmd
```

## Roadmap

- Scrolling
