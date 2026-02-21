# Changelog

All notable changes to this project will be documented in this file.

## [0.1.5] - 2026-02-21

### 🚀 Features

- *(keybindings)* Enter edit mode with `e`
- *(help)* Group help entries into categories

### 📚 Documentation

- *(readme)* Add a note that shift+↑/↓ may not work in all terminals
- *(readme,help)* Replace "event" with "message/note" in keybindings
- *(demo)* Update demo

### 🔧 Refactor

- *(app)* Split keybindings into separate widget IDs
- *(clippy)* Define some lint exceptions globally

### Build

- *(deps)* Update dependencies

## [0.1.4] - 2026-02-20

### 🚀 Features

- *(events)* Add support for notes
- *(events)* Change 'e' to 'm' for add message keybinding
- *(events)* Render with filled background
- *(export)* Extend notification time to 1s
- *(events)* Use single line height for notes
- *(events)* Insert messages and notes after selected event
- *(keybindings)* Insert messages/notes before or after selected event
- *(keybindings)* Move events/participants with shift+arrow
- *(help)* Reorder help entries

### 🐛 Bug Fixes

- *(lint)* Satisfy clippy
- *(ci)* Fail lint on warning

### 📚 Documentation

- *(demo)* Update demo
- *(demo)* Update demo

## [0.1.3] - 2026-02-19

### 🚀 Features

- *(navigation)* Add arrow key navigation
- *(app)* Add confirmation dialog before clearing diagram
- *(app)* Add event editing with Enter key
- *(app)* Add r keybinding to rename event message
- *(ui)* Make delete confirmation dialog clearer
- *(app)* Add rename support for participants

### 🐛 Bug Fixes

- *(lint)* Satisfy clippy
- *(clear)* Show confirmation dialog only if diagram exists

### 📚 Documentation

- *(readme)* Add badges
- *(readme)* Center header
- *(demo)* Update demo

### 🔧 Refactor

- *(keybindings)* Rework navigation

## [0.1.2] - 2026-02-18

### 🚀 Features

- *(nix)* Add nix flake for installation
- *(ui)* Set event arrow direction with H / L
- *(keybindings)* Make j/k navigate through participants
- *(help)* Shorten keys
- *(scroll)* Add scrolling of events
- *(scroll)* Add scrollbar
- *(scroll)* Show scroll inidicators oon lifelines
- *(selection)* Select new event after it is added
- *(help)* Show keys compactly formatted

### 🐛 Bug Fixes

- *(lint)* Satisfy clippy
- *(selection)* Select participant first if nothing is selected
- *(lint)* Satisfy clippy

### 📚 Documentation

- *(readme)* Add Homebrew installation instructions
- *(readme)* Make installation section collapsible
- *(demo)* Update demo tape and gif
- *(readme)* Update readme with newest changes
- *(readme)* Minimize installation instructions

### 🔧 Refactor

- *(render)* Simplify render code
- *(keybindings)* Simplify selection navigation
- *(render)* Pass world to render functions
- *(scroll)* Move constants to common mod
- *(imports)* Remove unused import
- *(keys)* Use uppercase keys for major action

### ⚙️ Miscellaneous Tasks

- *(nix)* Add flake.lock

## [0.1.1] - 2026-02-16

### 🚀 Features

- *(mermaid)* Add mermaid import
- *(loopback)* Add self-message support with loop-back

### 🐛 Bug Fixes

- *(lint)* Satisfy clippy

### 📚 Documentation

- *(README)* Organize sections
- *(README)* Add installation section

### 🔧 Refactor

- *(models)* Simplify sequence diagram and selection

### ⚙️ Miscellaneous Tasks

- *(gitignore)* Add exported diagram
- *(release)* Extract crates.io publish to separate job

### Build

- *(cd)* Release with binaries

## [0.1.0] - 2026-02-16

### 🚀 Features

- *(ui)* Add first version of UI
- *(export)* Export to mermaid
- *(nav)* Navigate with hjkl between participats/events
- *(input)* Swap events with J/K
- *(ui)* Decrease message spacing

### 🐛 Bug Fixes

- *(lint)* Satisfy clippy
- *(export)* Add trailing new line to mermaid export

### 📚 Documentation

- *(README)* Add demo tape and README
- *(demo)* Update demo
- *(demo)* Update demo
- *(README)* Add roadmap
- *(demo)* Use onedark theme

### ⚙️ Miscellaneous Tasks

- Add release changelog to gitignore
- Add metadata to cargo.toml

### Build

- *(ci)* Add CI
- *(ci)* Add git cliff
- *(deps)* Import tui-world from crates.io


