# Tuigram

A TUI sequence diagram editor.

![Demo](demo/demo.gif)

## Usage

```
cargo run --release
```

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

## Export

Press `m` to export the diagram to `diagram.mmd` in Mermaid format.

## Roadmap

- Scrolling
- Persistence
- Undo/redo
