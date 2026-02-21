#![allow(clippy::cast_possible_truncation)]

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use tui_world::{Keybindings, WidgetId};

use crate::theme::Theme;

const HELP_WIDTH: u16 = 44;

struct HelpEntry {
    keys: &'static str,
    description: &'static str,
}

struct HelpSection {
    title: &'static str,
    entries: &'static [HelpEntry],
}

const HELP_SECTIONS: &[HelpSection] = &[
    HelpSection {
        title: "[Navigation]",
        entries: &[
            HelpEntry {
                keys: "h/l, ←/→",
                description: "Select participant",
            },
            HelpEntry {
                keys: "j/k, ↓/↑",
                description: "Select message/note",
            },
        ],
    },
    HelpSection {
        title: "[Insert]",
        entries: &[
            HelpEntry {
                keys: "p",
                description: "Add participant",
            },
            HelpEntry {
                keys: "m/M",
                description: "Insert message after/before",
            },
            HelpEntry {
                keys: "n/N",
                description: "Insert note after/before",
            },
        ],
    },
    HelpSection {
        title: "[Edit]",
        entries: &[
            HelpEntry {
                keys: "e, Enter",
                description: "Edit selected",
            },
            HelpEntry {
                keys: "r",
                description: "Rename selected",
            },
            HelpEntry {
                keys: "d",
                description: "Delete selected",
            },
            HelpEntry {
                keys: "C",
                description: "Clear diagram",
            },
        ],
    },
    HelpSection {
        title: "[Move]",
        entries: &[
            HelpEntry {
                keys: "H/L, ⇧←/→",
                description: "Move participant",
            },
            HelpEntry {
                keys: "J/K, ⇧↓/↑",
                description: "Move message/note up/down",
            },
        ],
    },
    HelpSection {
        title: "[Other]",
        entries: &[
            HelpEntry {
                keys: "E",
                description: "Export to Mermaid",
            },
            HelpEntry {
                keys: "?",
                description: "Toggle help",
            },
            HelpEntry {
                keys: "Ctrl+c",
                description: "Quit",
            },
        ],
    },
];

pub fn render_help(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    _keybindings: &Keybindings,
    _active: &[WidgetId],
) {
    let mut lines: Vec<Line> = Vec::new();

    for (i, section) in HELP_SECTIONS.iter().enumerate() {
        if i > 0 {
            lines.push(Line::raw(""));
        }

        lines.push(Line::from(Span::styled(section.title, theme.muted)));

        for entry in section.entries {
            let spans = vec![
                Span::styled(format!(" {:>10}", entry.keys), theme.key),
                Span::raw("  "),
                Span::styled(entry.description, theme.text),
            ];
            lines.push(Line::from(spans));
        }
    }

    let popup_height = (lines.len() as u16 + 2).min(area.height.saturating_sub(2));

    let popup_area = centered_rect(HELP_WIDTH, popup_height, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Help ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(theme.border);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let [area] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);
    area
}
