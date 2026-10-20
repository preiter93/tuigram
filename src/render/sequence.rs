use ratatui::{
    Frame,
    buffer::{Buffer, Cell},
    layout::{Alignment, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, Widget},
};
use tui_world::World;

use crate::{
    core::{
        ArrowStyle, BoxColor, DEFAULT_BOX_COLOR, Event, Fragment, NotePosition, Participant,
        SequenceDiagram,
    },
    theme::Theme,
    ui::{
        EditorMode, EditorState, FIRST_MESSAGE_OFFSET, HEADER_HEIGHT, Selection,
        scroll::ScrollState,
    },
};

const HEADER_TOP: u16 = 1;
const ACTOR_GLYPH: &str = "☺ ";
const FRAME_PADDING: u16 = 3;
const SELF_LOOP_WIDTH: u16 = 4;
const MAX_CANVAS_WIDTH: u16 = 4096;

pub fn render_sequence(f: &mut Frame, outer_area: Rect, world: &mut World) {
    let selection = world.get::<EditorState>().selection;
    let area = outer_area.inner(Margin::new(0, 1));

    world.get_mut::<ScrollState>().set_viewport(area.height);

    let visible_event = match selection {
        Selection::Event(idx) => Some(idx),
        Selection::Fragment(idx) => world
            .get::<SequenceDiagram>()
            .fragments
            .get(idx)
            .map(|f| f.start),
        _ => None,
    };
    if let Some(idx) = visible_event {
        let diagram = world.get::<SequenceDiagram>().clone();
        world.get_mut::<ScrollState>().ensure_visible(idx, &diagram);
    }

    let theme = world.get::<Theme>();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border);
    f.render_widget(block, outer_area);

    if area.is_empty() {
        return;
    }

    let (participants, canvas_width) = participant_layout(area, world.get::<SequenceDiagram>());
    let canvas = Rect {
        width: canvas_width,
        ..area
    };
    let mut buf = Buffer::empty(canvas);

    let lifeline_start = area.y + HEADER_HEIGHT;
    let rows = event_rows(world, lifeline_start + FIRST_MESSAGE_OFFSET);
    render_box_outlines(&mut buf, canvas, world, &participants, &rows);
    render_participants(&mut buf, canvas, world, &participants);
    render_lifelines(&mut buf, canvas, world, &participants, lifeline_start);
    render_fragments(&mut buf, canvas, world, &participants, &rows);
    render_events(&mut buf, world, &participants, &rows);

    let (content_left, content_right) = content_columns(&buf);
    let max_offset = (content_right + 3)
        .saturating_sub(area.width)
        .min(canvas_width.saturating_sub(area.width));
    if let Some((left, right)) = selection_span(canvas, world, &participants) {
        let scroll = world.get_mut::<ScrollState>();
        scroll.follow_x(left, right, area.width, max_offset);

        // Don't leave blank columns hidden when the span fits without scrolling.
        if scroll.x_offset < content_left && right + 2 <= area.width {
            scroll.x_offset = 0;
        }

        // Likewise at the right end: keep a blank column before the border.
        if scroll.x_offset > 0 && right >= content_right && left > max_offset {
            scroll.x_offset = max_offset;
        }
    }
    let offset = world.get::<ScrollState>().x_offset.min(max_offset);
    world.get_mut::<ScrollState>().x_offset = offset;

    copy_visible_columns(f, &buf, area, offset);
    let hidden_left = offset >= content_left && offset > 0;
    let hidden_right = offset + area.width.saturating_sub(2) < content_right;
    render_scroll_markers(f, area, world, hidden_left, hidden_right);
    render_scrollbar(f, area, world);
}

/// Canvas columns (inclusive) the selection covers, if it has any.
fn selection_span(canvas: Rect, world: &World, positions: &[u16]) -> Option<(u16, u16)> {
    let diagram = world.get::<SequenceDiagram>();

    match world.get::<EditorState>().selection {
        Selection::Participant(i) => {
            let participant = diagram.participants.get(i)?;
            let (x, width) = header_span(canvas, positions[i], participant);
            Some((x, x + width - 1))
        }
        Selection::Box(i) => box_spans(canvas, world, positions)
            .get(i)
            .copied()
            .flatten(),
        Selection::Event(i) => {
            let event = diagram.events.get(i)?;
            Some(event_extent(event, &event_text(diagram, i), positions))
        }
        Selection::Fragment(i) => {
            let frame = diagram.fragments.get(i)?;
            let (left, width, _) = frame_bounds(canvas, diagram, frame, positions)?;
            Some((left, left + width - 1))
        }
        Selection::None => None,
    }
}

/// First and last canvas column (outside the border column) that has anything drawn.
fn content_columns(buf: &Buffer) -> (u16, u16) {
    let area = buf.area;
    let is_blank = |x: u16| (area.top()..area.bottom()).all(|y| buf[(x, y)] == Cell::EMPTY);
    let columns = area.x + 1..area.right();

    let left = columns.clone().find(|&x| !is_blank(x)).unwrap_or(0);
    let right = columns.rev().find(|&x| !is_blank(x)).unwrap_or(0);

    (left, right)
}

fn render_scroll_markers(
    f: &mut Frame,
    area: Rect,
    world: &World,
    hidden_left: bool,
    hidden_right: bool,
) {
    if area.width < 3 {
        return;
    }

    let style = world.get::<Theme>().muted;
    let y = area.y + HEADER_TOP + 1;

    if hidden_left {
        f.buffer_mut().set_string(area.x, y, "‹", style);
    }
    if hidden_right {
        f.buffer_mut().set_string(area.right() - 1, y, "›", style);
    }
}

fn render_scrollbar(f: &mut Frame, area: Rect, world: &World) {
    let diagram = world.get::<SequenceDiagram>();
    let scroll = world.get::<ScrollState>();

    if !scroll.needs_scroll(diagram) {
        return;
    }

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
    let scrollbar_area = Rect {
        x: area.x,
        y: area.y + HEADER_HEIGHT,
        width: area.width,
        height: area.height.saturating_sub(HEADER_HEIGHT),
    };
    let mut scrollbar_state = scroll.scrollbar_state(diagram);
    f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
}

/// Copies the canvas columns shown inside the border, shifted left by `offset`.
fn copy_visible_columns(f: &mut Frame, canvas: &Buffer, area: Rect, offset: u16) {
    let first = area.x + 1;
    let last = area.right().saturating_sub(1);

    for y in area.y..area.bottom() {
        for x in first..last {
            let Some(cell) = canvas.cell((x + offset, y)) else {
                continue;
            };
            if let Some(target) = f.buffer_mut().cell_mut((x, y)) {
                *target = cell.clone();

                // A wide glyph in the last column would spill onto the border.
                if x + 1 == last && text_width(cell.symbol()) > 1 {
                    target.set_symbol(" ");
                }
            }
        }
    }
}

/// Lifeline x of every participant and the canvas width the diagram needs.
fn participant_layout(area: Rect, diagram: &SequenceDiagram) -> (Vec<u16>, u16) {
    let count = diagram.participants.len();
    let gaps = column_gaps(diagram, area.width);
    let mut spacing = area.width / (count.max(1) as u16 + 1);

    let total = |spacing: u16| -> u32 { gaps.iter().map(|&g| u32::from(spacing.max(g))).sum() };
    while spacing > 0 && total(spacing) > u32::from(area.width) {
        spacing -= 1;
    }

    let mut positions = Vec::with_capacity(count);
    let mut x = 0;
    for &gap in &gaps[..count] {
        x = u16::saturating_add(x, spacing.max(gap));
        positions.push(x);
    }
    let mut content_width = x.saturating_add(spacing.max(gaps[count]));

    // Wide notes and labels that the gap rules skip still need to be reachable.
    for (i, event) in diagram.events.iter().enumerate() {
        let (_, right) = event_extent(event, &event_text(diagram, i), &positions);
        content_width = content_width.max(right.saturating_add(3));
    }

    let canvas_width = area.width.max(content_width.min(MAX_CANVAS_WIDTH));
    (positions, canvas_width)
}

/// Minimum room left of every participant, plus one entry for the room right of the last.
fn column_gaps(diagram: &SequenceDiagram, max_gap: u16) -> Vec<u16> {
    let count = diagram.participants.len();
    let mut gaps = vec![0u16; count + 1];
    if count == 0 {
        return gaps;
    }

    let widths: Vec<u16> = diagram
        .participants
        .iter()
        .map(|p| text_width(&header_text(p)) + 4)
        .collect();
    let box_of = |i: usize| {
        diagram
            .boxes
            .iter()
            .position(|b| b.start <= i && i <= b.end)
    };

    gaps[0] = widths[0] / 2 + 2;
    gaps[count] = widths[count - 1] / 2 + 2;
    for i in 1..count {
        gaps[i] = u16::midpoint(widths[i - 1], widths[i]) + 2;
        if box_of(i - 1) != box_of(i) {
            gaps[i] += 3;
        }
    }
    if box_of(0).is_some() {
        gaps[0] += 2;
    }
    if box_of(count - 1).is_some() {
        gaps[count] += 2;
    }

    for (i, event) in diagram.events.iter().enumerate() {
        let text = text_width(&event_text(diagram, i));
        match event {
            Event::Message { from, to, .. } if from == to => {
                gaps[from + 1] = gaps[from + 1].max(SELF_LOOP_WIDTH + text + 1);
            }
            Event::Message { from, to, .. } if from.abs_diff(*to) == 1 => {
                let right = *from.max(to);
                gaps[right] = gaps[right].max(text + 4);
            }
            Event::Message { .. } => {}
            Event::Note {
                position,
                participant_start,
                participant_end,
                ..
            } => {
                if participant_start != participant_end {
                    continue;
                }
                let i = *participant_start;
                match position {
                    NotePosition::Right => gaps[i + 1] = gaps[i + 1].max(text + 7),
                    NotePosition::Left => gaps[i] = gaps[i].max(text + 7),
                    NotePosition::Over => {
                        let half = u16::midpoint(text, 4) + 2;
                        gaps[i] = gaps[i].max(half);
                        gaps[i + 1] = gaps[i + 1].max(half);
                    }
                }
            }
        }
    }

    // Room for the border column, and for frames drawn around edge events.
    gaps[count] += 1;
    if !diagram.fragments.is_empty() {
        gaps[0] += FRAME_PADDING;
        gaps[count] += FRAME_PADDING;
    }

    for gap in &mut gaps {
        *gap = (*gap).min(max_gap);
    }
    gaps
}

/// Left edge and width of a participant's header box.
fn header_span(area: Rect, center: u16, participant: &Participant) -> (u16, u16) {
    let width = text_width(&header_text(participant)) + 4;
    let x = center
        .saturating_sub(width / 2)
        .min(area.width.saturating_sub(width));
    (x, width)
}

fn render_participants(buf: &mut Buffer, area: Rect, world: &World, positions: &[u16]) {
    let diagram = world.get::<SequenceDiagram>();
    let selection = world.get::<EditorState>().selection;
    let theme = world.get::<Theme>();

    for (i, participant) in diagram.participants.iter().enumerate() {
        let style = if selection == Selection::Participant(i) {
            theme.selected
        } else {
            theme.text
        };

        let (x, width) = header_span(area, positions[i], participant);

        Paragraph::new(header_text(participant))
            .alignment(Alignment::Center)
            .style(style)
            .block(Block::default().borders(Borders::ALL).border_style(style))
            .render(
                Rect {
                    x,
                    y: area.y + HEADER_TOP,
                    width,
                    height: 3,
                },
                buf,
            );
    }
}

fn header_text(p: &Participant) -> String {
    if p.actor {
        format!("{ACTOR_GLYPH}{}", p.display_name())
    } else {
        p.display_name().to_string()
    }
}

/// Columns (inclusive) a box outline covers: a border column and a blank column
/// around the headers. Neighbouring boxes never overlap.
fn box_spans(area: Rect, world: &World, positions: &[u16]) -> Vec<Option<(u16, u16)>> {
    let diagram = world.get::<SequenceDiagram>();
    let min_left = area.x + 1;
    let max_right = area.right().saturating_sub(2);
    let mut previous_right: Option<u16> = None;
    let mut spans = Vec::new();

    for b in &diagram.boxes {
        if b.end >= positions.len() {
            spans.push(None);
            continue;
        }

        let (first_x, _) = header_span(area, positions[b.start], &diagram.participants[b.start]);
        let (last_x, last_w) = header_span(area, positions[b.end], &diagram.participants[b.end]);

        let mut left = (area.x + first_x).saturating_sub(2).max(min_left);
        if let Some(right) = previous_right {
            left = left.max(right + 1);
        }
        let right = (area.x + last_x + last_w + 1).min(max_right);

        if right < left + 1 {
            spans.push(None);
            continue;
        }

        // Room up to the neighbouring headers, leaving room for their box border.
        let lower = match b.start.checked_sub(1) {
            Some(prev) => {
                let (x, w) = header_span(area, positions[prev], &diagram.participants[prev]);
                area.x + x + w + 2
            }
            None => min_left,
        };
        let upper = match diagram.participants.get(b.end + 1) {
            Some(next) => {
                let (x, _) = header_span(area, positions[b.end + 1], next);
                (area.x + x).saturating_sub(3)
            }
            None => max_right,
        };
        let lower = lower
            .max(previous_right.map_or(min_left, |r| r + 1))
            .min(left);
        let upper = upper.min(max_right).max(right);

        let (left, right) = widen_for_label(left, right, &b.label, lower, upper);

        previous_right = Some(right);
        spans.push(Some((left, right)));
    }

    spans
}

/// Grows a span symmetrically so `╭─ label ─╮` fits, as far as the limits allow.
fn widen_for_label(left: u16, right: u16, label: &str, lower: u16, upper: u16) -> (u16, u16) {
    if label.is_empty() {
        return (left, right);
    }

    let wanted = text_width(label) + 6;
    let width = right - left + 1;
    if width >= wanted {
        return (left, right);
    }

    let extra = wanted - width;
    let new_left = left.saturating_sub(extra / 2).max(lower);
    let new_right = (right + extra - extra / 2).min(upper);
    if new_right - new_left + 1 >= wanted {
        return (new_left, new_right);
    }

    // One side ran out of room: give the rest to the other side.
    let new_left = (new_right + 1).saturating_sub(wanted).max(lower);
    let new_right = (new_left + wanted - 1).min(upper);
    (new_left, new_right)
}

fn render_box_outlines(
    buf: &mut Buffer,
    area: Rect,
    world: &World,
    positions: &[u16],
    rows: &[(usize, u16)],
) {
    let diagram = world.get::<SequenceDiagram>();
    let theme = world.get::<Theme>();
    let selection = world.get::<EditorState>().selection;
    let area_bottom = area.y + area.height.saturating_sub(1);

    let event_count = diagram.event_count();
    let has_below = world.get::<ScrollState>().visible_range(diagram).end < event_count;
    let content_end = match rows.last() {
        Some(&(i, y)) => {
            let has_header = diagram.fragments.iter().any(|f| f.start == i);
            y + diagram.event_height(i) - u16::from(has_header)
        }
        None => area.y + HEADER_HEIGHT + FIRST_MESSAGE_OFFSET,
    };
    let bottom = if has_below {
        area_bottom
    } else {
        content_end.min(area_bottom)
    };

    let top = area.y;
    if bottom < top + HEADER_HEIGHT {
        return;
    }

    let spans = box_spans(area, world, positions);

    for (box_idx, (b, span)) in diagram.boxes.iter().zip(spans).enumerate() {
        let Some((left, right)) = span else {
            continue;
        };

        let style = if selection == Selection::Box(box_idx) {
            theme.selected.add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(box_color(b.color.unwrap_or(DEFAULT_BOX_COLOR)))
        };
        let width = right - left + 1;

        Paragraph::new(box_top_edge(&b.label, width))
            .style(style)
            .render(
                Rect {
                    x: left,
                    y: top,
                    width,
                    height: 1,
                },
                buf,
            );

        for y in top + 1..bottom {
            for x in [left, right] {
                Paragraph::new("│").style(style).render(
                    Rect {
                        x,
                        y,
                        width: 1,
                        height: 1,
                    },
                    buf,
                );
            }
        }

        let dashes = "─".repeat(usize::from(width - 2));
        Paragraph::new(format!("╰{dashes}╯")).style(style).render(
            Rect {
                x: left,
                y: bottom,
                width,
                height: 1,
            },
            buf,
        );
    }
}

fn box_top_edge(label: &str, width: u16) -> String {
    let inner = usize::from(width - 2);
    // "─ " before and " ─" after the label
    let label_room = width.saturating_sub(6);
    if label.is_empty() || label_room == 0 {
        return format!("╭{}╮", "─".repeat(inner));
    }

    let label = truncate_to_width(label, label_room);
    let dashes = inner.saturating_sub(text_width(&label) as usize + 3);
    format!("╭─ {label} {}╮", "─".repeat(dashes))
}

pub fn box_color(color: BoxColor) -> Color {
    match color {
        BoxColor::Blue => Color::Rgb(100, 150, 220),
        BoxColor::Green => Color::Rgb(80, 180, 100),
        BoxColor::Red => Color::Rgb(220, 80, 80),
        BoxColor::Yellow => Color::Rgb(200, 180, 50),
        BoxColor::Orange => Color::Rgb(220, 130, 50),
        BoxColor::Purple => Color::Rgb(170, 80, 200),
        BoxColor::Aqua => Color::Rgb(60, 190, 200),
        BoxColor::Gray => Color::Rgb(150, 150, 150),
    }
}

fn render_lifelines(
    buf: &mut Buffer,
    area: Rect,
    world: &World,
    participants: &[u16],
    lifeline_start: u16,
) {
    let diagram = world.get::<SequenceDiagram>();
    let scroll = world.get::<ScrollState>();
    let theme = world.get::<Theme>();
    let event_count = diagram.event_count();

    let visible_range = scroll.visible_range(diagram);
    let has_above = visible_range.start > 0;
    let has_below = visible_range.end < event_count;

    for &x in participants {
        let xi = x.min(area.width.saturating_sub(1));
        for y in lifeline_start..area.y + area.height {
            Paragraph::new("│").style(theme.text).render(
                Rect {
                    x: xi,
                    y,
                    width: 1,
                    height: 1,
                },
                buf,
            );
        }

        // Show scroll indicators on lifelines
        if has_above {
            Paragraph::new("⋮").style(theme.muted).render(
                Rect {
                    x: xi,
                    y: lifeline_start,
                    width: 1,
                    height: 1,
                },
                buf,
            );
        }

        if has_below {
            Paragraph::new("⋮").style(theme.muted).render(
                Rect {
                    x: xi,
                    y: area.y + area.height - 1,
                    width: 1,
                    height: 1,
                },
                buf,
            );
        }
    }
}

/// Anchor row of every visible event, leaving room for frame header rows.
fn event_rows(world: &World, base_y: u16) -> Vec<(usize, u16)> {
    let diagram = world.get::<SequenceDiagram>();
    let visible_range = world.get::<ScrollState>().visible_range(diagram);

    let mut y = base_y;
    let mut rows = Vec::new();

    for i in visible_range {
        let has_header = diagram.fragments.iter().any(|f| f.start == i);
        if has_header {
            y += 1;
        }
        rows.push((i, y));
        y += diagram.event_height(i) - u16::from(has_header);
    }

    rows
}

fn render_fragments(
    buf: &mut Buffer,
    area: Rect,
    world: &World,
    participants: &[u16],
    rows: &[(usize, u16)],
) {
    let diagram = world.get::<SequenceDiagram>();
    let theme = world.get::<Theme>();
    let (Some(&(first_visible, _)), Some(&(last_visible, _))) = (rows.first(), rows.last()) else {
        return;
    };

    let first_row = area.y + HEADER_HEIGHT;
    let last_row = area.y + area.height - 1;
    let anchor = |idx: usize| rows.iter().find(|(i, _)| *i == idx).map(|(_, y)| *y);
    let selection = world.get::<EditorState>().selection;

    for (frame_idx, frame) in diagram.fragments.iter().enumerate() {
        let (border_style, title_style) = if selection == Selection::Fragment(frame_idx) {
            (theme.selected, theme.selected)
        } else {
            (theme.muted, theme.accent)
        };

        if frame.end < first_visible || frame.start > last_visible {
            continue;
        }

        let header_y = anchor(frame.start).map(|y| y - 2);
        let footer_y = anchor(frame.end).map(|y| y + diagram.events[frame.end].height() - 1);

        let Some((left, width, title)) = frame_bounds(area, diagram, frame, participants) else {
            continue;
        };
        let title_width = text_width(&title);
        let right = left + width - 1;

        let top = header_y.unwrap_or(first_row);
        let bottom = footer_y.unwrap_or(last_row);

        for y in top.max(first_row)..=bottom.min(last_row) {
            let row = Rect {
                x: left,
                y,
                width,
                height: 1,
            };

            if Some(y) == header_y {
                let dashes = usize::from(width.saturating_sub(title_width + 4).max(1));
                let line = Line::from(vec![
                    Span::styled("┌ ", border_style),
                    Span::styled(title.as_str(), title_style),
                    Span::styled(format!(" {}┐", "─".repeat(dashes)), border_style),
                ]);
                Paragraph::new(line).render(row, buf);
            } else if Some(y) == footer_y {
                let dashes = "─".repeat(usize::from(width.saturating_sub(2)));
                Paragraph::new(format!("└{dashes}┘"))
                    .style(border_style)
                    .render(row, buf);
            } else {
                for x in [left, right] {
                    Paragraph::new("│")
                        .style(border_style)
                        .render(Rect { x, width: 1, ..row }, buf);
                }
            }
        }
    }
}

/// Left x, width and (possibly truncated) title of a frame, or None if it cannot fit.
fn frame_bounds(
    area: Rect,
    diagram: &SequenceDiagram,
    frame: &Fragment,
    participants: &[u16],
) -> Option<(u16, u16, String)> {
    let extents: Vec<(u16, u16)> = (frame.start..=frame.end)
        .map(|i| event_extent(&diagram.events[i], &event_text(diagram, i), participants))
        .collect();
    let min_x = extents.iter().map(|e| e.0).min().unwrap_or(0);
    let max_x = extents.iter().map(|e| e.1).max().unwrap_or(0);

    let min_left = area.x + 1;
    let max_right = area.right().saturating_sub(2);
    let available = (max_right + 1).saturating_sub(min_left);
    if available < 6 {
        return None;
    }

    let title = truncate_to_width(
        &frame_title(frame.kind.as_mermaid_str(), &frame.label),
        available - 5,
    );
    let desired_left = min_x.saturating_sub(FRAME_PADDING).max(min_left);
    let width = (max_x + FRAME_PADDING + 1)
        .saturating_sub(desired_left)
        .max(text_width(&title) + 5)
        .min(available);
    let left = desired_left.min(max_right + 1 - width);

    Some((left, width, title))
}

fn truncate_to_width(text: &str, max_width: u16) -> String {
    if text_width(text) <= max_width {
        return text.to_string();
    }

    let mut width = 0;
    let mut result = String::new();

    for c in text.chars() {
        width += text_width(&c.to_string());
        if width >= max_width {
            break;
        }
        result.push(c);
    }

    format!("{}…", result.trim_end())
}

fn frame_title(kind: &str, label: &str) -> String {
    if label.is_empty() {
        kind.to_string()
    } else {
        format!("{kind} [{label}]")
    }
}

fn render_events(buf: &mut Buffer, world: &World, participants: &[u16], rows: &[(usize, u16)]) {
    let diagram = world.get::<SequenceDiagram>();
    let editor = world.get::<EditorState>();
    let theme = world.get::<Theme>();

    let frame_range = frame_selection_range(editor);

    for &(i, y) in rows {
        let event = &diagram.events[i];
        let label = event_text(diagram, i);

        let style = if editor.selection == Selection::Event(i) {
            theme.selected
        } else if frame_range.as_ref().is_some_and(|range| range.contains(&i)) {
            theme.accent
        } else {
            theme.text
        };

        match event {
            Event::Message {
                from,
                to,
                style: arrow_style,
                ..
            } => {
                render_message(
                    buf,
                    participants[*from],
                    participants[*to],
                    &label,
                    *arrow_style,
                    y,
                    style,
                );
            }
            Event::Note {
                position,
                participant_start,
                participant_end,
                ..
            } => {
                render_note(
                    buf,
                    participants,
                    *position,
                    *participant_start,
                    *participant_end,
                    &label,
                    y,
                    style,
                );
            }
        }
    }
}

fn frame_selection_range(editor: &EditorState) -> Option<std::ops::RangeInclusive<usize>> {
    if editor.mode != EditorMode::SelectFrameEnd {
        return None;
    }
    let (Some(start), Selection::Event(cursor)) = (editor.frame_start, editor.selection) else {
        return None;
    };
    Some(start.min(cursor)..=start.max(cursor))
}

fn line_glyph(arrow_style: ArrowStyle) -> char {
    if arrow_style.is_dashed() {
        '┄'
    } else {
        '─'
    }
}

fn head_glyph(arrow_style: ArrowStyle, pointing_right: bool) -> Option<char> {
    match arrow_style {
        ArrowStyle::SolidArrow | ArrowStyle::DashedArrow => {
            Some(if pointing_right { '▶' } else { '◀' })
        }
        ArrowStyle::Solid | ArrowStyle::Dashed => None,
        ArrowStyle::SolidCross | ArrowStyle::DashedCross => Some('×'),
        ArrowStyle::SolidAsync | ArrowStyle::DashedAsync => {
            Some(if pointing_right { '▷' } else { '◁' })
        }
    }
}

fn render_message(
    buf: &mut Buffer,
    from_x: u16,
    to_x: u16,
    text: &str,
    arrow_style: ArrowStyle,
    y: u16,
    style: ratatui::style::Style,
) {
    if from_x == to_x {
        let mut area = Rect {
            x: from_x,
            y: y.saturating_sub(1),
            width: SELF_LOOP_WIDTH,
            height: 1,
        };

        let line = line_glyph(arrow_style);
        let head = head_glyph(arrow_style, false).unwrap_or(line);

        Paragraph::new(format!("{line}{line}{line}┐"))
            .style(style)
            .render(area, buf);

        area.y = y;
        // Start after the lifeline column so the lifeline stays visible.
        let middle = Rect {
            x: area.x + 1,
            width: area.width - 1,
            ..area
        };
        Paragraph::new("  │").style(style).render(middle, buf);

        area.y = y + 1;
        Paragraph::new(format!("{head}{line}{line}┘"))
            .style(style)
            .render(area, buf);

        area.x = from_x + SELF_LOOP_WIDTH;
        area.y = y;
        area.width = text_width(text);
        Paragraph::new(text).style(style).render(area, buf);
    } else {
        let start = from_x.min(to_x);
        let end = from_x.max(to_x);
        let len = end - start;

        let line = line_glyph(arrow_style);
        let pointing_right = from_x < to_x;
        let mut arrow = line.to_string().repeat(len as usize);
        let head = head_glyph(arrow_style, pointing_right).unwrap_or(line);
        if pointing_right {
            arrow.push(head);
        } else {
            arrow.insert(0, head);
        }

        Paragraph::new(Line::from(arrow)).style(style).render(
            Rect {
                x: start,
                y,
                width: len + 1,
                height: 1,
            },
            buf,
        );

        let width = text_width(text);
        let text_start = message_text_start(start, len, width);

        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(style)
            .render(
                Rect {
                    x: text_start,
                    y: y.saturating_sub(1),
                    width,
                    height: 1,
                },
                buf,
            );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_note(
    buf: &mut Buffer,
    participants: &[u16],
    position: NotePosition,
    participant_start: usize,
    participant_end: usize,
    text: &str,
    y: u16,
    style: ratatui::style::Style,
) {
    let (x, width) = note_span(
        position,
        participants[participant_start],
        participants[participant_end],
        text_width(text),
    );

    let area = Rect {
        x,
        y: y.saturating_sub(1),
        width,
        height: 1,
    }
    .intersection(buf.area);

    Clear.render(area, buf);
    Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(style.reversed())
        .render(area, buf);
}

fn event_text(diagram: &SequenceDiagram, idx: usize) -> String {
    let text = match &diagram.events[idx] {
        Event::Message { text, .. } | Event::Note { text, .. } => text,
    };
    match diagram.message_number(idx) {
        Some(n) => format!("{n}. {text}"),
        None => text.clone(),
    }
}

/// Left and right x (inclusive) of everything an event draws.
fn event_extent(event: &Event, text: &str, participants: &[u16]) -> (u16, u16) {
    match event {
        Event::Message { from, to, .. } => {
            let (from_x, to_x) = (participants[*from], participants[*to]);
            let width = text_width(text);

            if from_x == to_x {
                let right = from_x + SELF_LOOP_WIDTH + width.max(1) - 1;
                return (from_x, right.max(from_x + SELF_LOOP_WIDTH - 1));
            }

            let (start, end) = (from_x.min(to_x), from_x.max(to_x));
            let text_start = message_text_start(start, end - start, width);
            let text_end = text_start + width.max(1) - 1;
            (start.min(text_start), end.max(text_end))
        }
        Event::Note {
            position,
            participant_start,
            participant_end,
            ..
        } => {
            let (x, width) = note_span(
                *position,
                participants[*participant_start],
                participants[*participant_end],
                text_width(text),
            );
            (x, x + width - 1)
        }
    }
}

fn message_text_start(start: u16, len: u16, text_width: u16) -> u16 {
    (start + len.div_ceil(2)).saturating_sub(text_width / 2)
}

/// Left x and width of a note, given the text width.
fn note_span(position: NotePosition, start_x: u16, end_x: u16, text_width: u16) -> (u16, u16) {
    match position {
        NotePosition::Right => (start_x.saturating_add(2), text_width + 4),
        NotePosition::Left => {
            let w = text_width + 4;
            (start_x.saturating_sub(w + 1), w)
        }
        NotePosition::Over => {
            let min_x = start_x.min(end_x);
            let max_x = start_x.max(end_x);
            let span = max_x.saturating_sub(min_x);
            let w = span.max(text_width + 2) + 2;
            let x = if span > 0 {
                min_x.saturating_sub(1)
            } else {
                min_x.saturating_sub(w / 2)
            };
            (x, w)
        }
    }
}

fn text_width(text: &str) -> u16 {
    Line::from(text).width() as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(source: &str, width: u16) -> (Vec<u16>, u16) {
        let diagram = SequenceDiagram::from_mermaid(source).unwrap();
        participant_layout(Rect::new(0, 0, width, 20), &diagram)
    }

    fn eight_participants(messages: &str) -> String {
        let names = (1..=8)
            .map(|i| format!("participant Participant{i}\n"))
            .collect::<Vec<String>>()
            .concat();
        format!("sequenceDiagram\n{names}{messages}")
    }

    #[test]
    fn short_diagram_keeps_even_spacing() {
        let source = "sequenceDiagram\nA->>B: hi\nB->>C: yo\n";

        assert_eq!(layout(source, 80), (vec![20, 40, 60], 80));
    }

    #[test]
    fn crowded_headers_widen_the_canvas() {
        let messages = (1..8)
            .map(|i| {
                format!(
                    "Participant{i}->>Participant{}: a long message label\n",
                    i + 1
                )
            })
            .collect::<Vec<String>>()
            .concat();
        let source = eight_participants(&messages);
        let diagram = SequenceDiagram::from_mermaid(&source).unwrap();

        let (positions, width) = layout(&source, 80);
        assert!(width > 80);

        let label = text_width("a long message label");
        for pair in positions.windows(2) {
            assert!(pair[1] - pair[0] >= label + 2);
        }
        for (i, pair) in positions.windows(2).enumerate() {
            let (x0, w0) = header_span(
                Rect::new(0, 0, width, 20),
                pair[0],
                &diagram.participants[i],
            );
            let (x1, _) = header_span(
                Rect::new(0, 0, width, 20),
                pair[1],
                &diagram.participants[i + 1],
            );
            assert!(x0 + w0 < x1, "headers {i} and {} overlap", i + 1);
        }
    }

    #[test]
    fn last_participant_keeps_room_for_self_message_and_note() {
        let source = eight_participants(
            "Participant8->>Participant8: loop back\nNote right of Participant8: remember this\n",
        );

        let (positions, width) = layout(&source, 80);
        let last = *positions.last().unwrap();
        let note = text_width("remember this");

        assert!(last + 2 + note + 4 < width);
        assert!(last + SELF_LOOP_WIDTH + text_width("loop back") < width);
    }

    #[test]
    fn widened_message_label_has_blank_columns_on_both_sides() {
        let source =
            "sequenceDiagram\nparticipant A\nparticipant B\nA->>B: a long label that needs room\n";
        let (positions, _) = layout(source, 40);

        let label = text_width("a long label that needs room");
        assert!(positions[1] - positions[0] >= label + 4);
    }

    #[test]
    fn label_that_fits_between_headers_does_not_widen_canvas() {
        let source =
            "sequenceDiagram\nparticipant A\nparticipant B\nA->>B: 123456789012345678901234x\n";

        assert_eq!(layout(source, 80).1, 80);
    }

    #[test]
    fn wide_glyph_in_last_visible_column_is_blanked() {
        use ratatui::{Terminal, backend::TestBackend};

        let area = Rect::new(0, 0, 10, 3);
        let mut canvas = Buffer::empty(area);
        canvas.set_string(8, 1, "用", Style::default());

        let mut terminal = Terminal::new(TestBackend::new(10, 3)).unwrap();
        terminal
            .draw(|f| copy_visible_columns(f, &canvas, area, 0))
            .unwrap();

        assert_eq!(terminal.backend().buffer()[(8, 1)].symbol(), " ");
    }
}
