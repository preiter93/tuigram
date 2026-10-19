#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

mod app;
mod core;
mod render;
mod theme;
mod ui;

use clap::Parser;
use crossterm::clipboard::CopyToClipboard;
use ratatui::crossterm::{
    event::{self, Event as CEvent, KeyEventKind},
    execute,
};
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};
use tui_world::prelude::*;

use crate::{core::SequenceDiagram, ui::EditorState};

/// A TUI sequence diagram editor
#[derive(Parser, Debug)]
#[command(name = "tuigram")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Mermaid (.mmd) file to open; created on save if missing
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Same as the positional FILE argument
    #[arg(short, long, value_name = "FILE", conflicts_with = "file")]
    import: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let path = args.file.or(args.import);

    let diagram = match &path {
        Some(path) => load_diagram(path)?,
        None => SequenceDiagram::new(),
    };

    run(diagram, path)
}

fn load_diagram(path: &Path) -> anyhow::Result<SequenceDiagram> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(SequenceDiagram::new()),
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Failed to read file '{}': {}",
                path.display(),
                e
            ));
        }
    };

    SequenceDiagram::from_mermaid(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse mermaid file '{}': {}", path.display(), e))
}

fn copy_pending_clipboard(world: &mut World) {
    let Some(text) = world.get_mut::<app::AppState>().pending_clipboard.take() else {
        return;
    };

    if let Err(e) = execute!(std::io::stdout(), CopyToClipboard::to_clipboard_from(text)) {
        world
            .get_mut::<EditorState>()
            .set_error(format!("Copy failed: {e}"));
    }
}

fn run(diagram: SequenceDiagram, path: Option<PathBuf>) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    execute!(std::io::stdout())?;

    let mut world = World::default();
    app::setup_world(&mut world, diagram, path);

    loop {
        terminal.draw(|frame| app::render(frame, &mut world))?;

        if event::poll(std::time::Duration::from_millis(16))?
            && let CEvent::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app::handle_key(&mut world, key);
        }

        copy_pending_clipboard(&mut world);

        if world.get::<app::AppState>().should_quit {
            break;
        }
    }

    execute!(std::io::stdout())?;
    ratatui::restore();

    Ok(())
}
