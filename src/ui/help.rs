#![allow(clippy::cast_possible_truncation)]

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use tui_world::{Keybindings, WidgetId};

use crate::theme::Theme;

const COLUMN_GAP: u16 = 2;

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
        title: "[Navigation/Mouse]",
        entries: &[
            HelpEntry {
                keys: "h/l, ←/→",
                description: "Select participant/box",
            },
            HelpEntry {
                keys: "j/k, ↓/↑",
                description: "Select message/note",
            },
            HelpEntry {
                keys: "g/G",
                description: "First/last message/note",
            },
            HelpEntry {
                keys: "Click",
                description: "Select (again: edit)",
            },
            HelpEntry {
                keys: "Wheel",
                description: "Scroll",
            },
            HelpEntry {
                keys: "Shift+Wheel",
                description: "Scroll sideways",
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
                description: "Message after/before",
            },
            HelpEntry {
                keys: "n/N",
                description: "Note after/before",
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
                keys: "Alt+Enter",
                description: "Line break in text",
            },
            HelpEntry {
                keys: "r",
                description: "Rename selected",
            },
            HelpEntry {
                keys: "t",
                description: "Toggle participant/actor",
            },
            HelpEntry {
                keys: "#",
                description: "Toggle autonumber",
            },
            HelpEntry {
                keys: "D",
                description: "Delete selected",
            },
            HelpEntry {
                keys: "a/A",
                description: "Cycle arrow style",
            },
            HelpEntry {
                keys: "C",
                description: "Clear diagram",
            },
            HelpEntry {
                keys: "u/U, Ctrl+r",
                description: "Undo/redo",
            },
        ],
    },
    HelpSection {
        title: "[Move]",
        entries: &[
            HelpEntry {
                keys: "H/L, ⇧←/→",
                description: "Move participant/box/arrow",
            },
            HelpEntry {
                keys: "J/K, ⇧↓/↑",
                description: "Move event/frame up/down",
            },
        ],
    },
    HelpSection {
        title: "[Box/Frame]",
        entries: &[
            HelpEntry {
                keys: "b",
                description: "Add box (h/l to extend)",
            },
            HelpEntry {
                keys: "B",
                description: "Remove box",
            },
            HelpEntry {
                keys: "f",
                description: "Add/edit frame (j/k)",
            },
            HelpEntry {
                keys: "F",
                description: "Remove frame",
            },
            HelpEntry {
                keys: "s",
                description: "Add section divider",
            },
        ],
    },
    HelpSection {
        title: "[Other]",
        entries: &[
            HelpEntry {
                keys: "S",
                description: "Save (Enter keeps path)",
            },
            HelpEntry {
                keys: "y",
                description: "Copy Mermaid to clipboard",
            },
            HelpEntry {
                keys: "?",
                description: "Show help (any key closes)",
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
    let split = split_index();
    let (left, right) = HELP_SECTIONS.split_at(split);
    let left_width = column_width(left);
    let right_width = column_width(right);
    let two_columns_width = left_width + COLUMN_GAP + right_width + 2;

    let block = Block::default()
        .title(" Help ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(theme.border);

    if area.width >= two_columns_width {
        let left_lines = section_lines(left, theme);
        let right_lines = section_lines(right, theme);
        let height = left_lines.len().max(right_lines.len()) as u16 + 2;
        let popup_area = centered_rect(two_columns_width, height.min(area.height), area);

        frame.render_widget(Clear, popup_area);
        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let [left_area, _, right_area] = Layout::horizontal([
            Constraint::Length(left_width),
            Constraint::Length(COLUMN_GAP),
            Constraint::Length(right_width),
        ])
        .areas(inner);
        frame.render_widget(Paragraph::new(left_lines), left_area);
        frame.render_widget(Paragraph::new(right_lines), right_area);
        return;
    }

    let lines = section_lines(HELP_SECTIONS, theme);
    let width = column_width(HELP_SECTIONS) + 2;
    let height = (lines.len() as u16 + 2).min(area.height);
    let popup_area = centered_rect(width.min(area.width), height, area);

    frame.render_widget(Clear, popup_area);
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);
    frame.render_widget(Paragraph::new(lines), inner);
}

fn section_height(section: &HelpSection) -> usize {
    section.entries.len() + 2
}

fn split_index() -> usize {
    let total: usize = HELP_SECTIONS.iter().map(section_height).sum();
    let mut height = 0;

    for (i, section) in HELP_SECTIONS.iter().enumerate() {
        if height * 2 >= total {
            return i;
        }
        height += section_height(section);
    }

    HELP_SECTIONS.len()
}

fn key_width(sections: &[HelpSection]) -> usize {
    sections
        .iter()
        .flat_map(|section| section.entries)
        .map(|entry| entry.keys.chars().count())
        .max()
        .unwrap_or(0)
}

fn column_width(sections: &[HelpSection]) -> u16 {
    let description_width = sections
        .iter()
        .flat_map(|section| section.entries)
        .map(|entry| entry.description.chars().count())
        .max()
        .unwrap_or(0);

    (1 + key_width(sections) + 2 + description_width) as u16
}

fn section_lines<'a>(sections: &'a [HelpSection], theme: &Theme) -> Vec<Line<'a>> {
    let key_width = key_width(sections);
    let mut lines = Vec::new();

    for (i, section) in sections.iter().enumerate() {
        if i > 0 {
            lines.push(Line::raw(""));
        }

        lines.push(Line::from(Span::styled(section.title, theme.muted)));

        for entry in section.entries {
            lines.push(Line::from(vec![
                Span::styled(format!(" {:>key_width$}", entry.keys), theme.key),
                Span::raw("  "),
                Span::styled(entry.description, theme.text),
            ]));
        }
    }

    lines
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn help_fits_80x24_with_all_sections() {
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let theme = Theme::default();
        let keybindings = Keybindings::default();

        terminal
            .draw(|frame| render_help(frame, frame.area(), &theme, &keybindings, &[]))
            .unwrap();

        let screen = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(screen.contains("[Other]"));
        assert!(screen.contains("Copy Mermaid to clipboard"));
        assert!(screen.contains("Quit"));
        assert!(screen.contains("Toggle autonumber"));
        assert!(screen.contains("Remove frame"));
        assert!(screen.contains("Add section divider"));
        assert!(screen.contains("g/G"));
        assert!(screen.contains("[Navigation/Mouse]"));
        assert!(screen.contains("Scroll sideways"));
    }
}
