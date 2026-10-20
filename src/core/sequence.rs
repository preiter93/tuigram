use super::models::{
    ArrowStyle, BoxColor, Event, Fragment, FragmentKind, NotePosition, Participant, ParticipantBox,
};
use anyhow::{Result, anyhow, bail};

const BLOCK_KEYWORDS: &[&str] = &[
    "alt", "else", "loop", "opt", "par", "and", "critical", "option", "break", "rect",
];
const OTHER_KEYWORDS: &[&str] = &["activate", "deactivate", "autonumber", "create", "destroy"];
const RESERVED_WORDS: &[&str] = &[
    "end",
    "box",
    "participant",
    "actor",
    "note",
    "sequenceDiagram",
];
const ARROWS: &[&str] = &["-->>", "->>", "--x", "-x", "--)", "-)", "-->", "->"];

#[derive(Default, Clone, PartialEq)]
pub struct SequenceDiagram {
    pub participants: Vec<Participant>,
    pub events: Vec<Event>,
    pub boxes: Vec<ParticipantBox>,
    pub fragments: Vec<Fragment>,
    pub autonumber: bool,
}

impl SequenceDiagram {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle_autonumber(&mut self) -> bool {
        self.autonumber = !self.autonumber;
        self.autonumber
    }

    pub fn message_number(&self, idx: usize) -> Option<usize> {
        if !self.autonumber || !matches!(self.events.get(idx)?, Event::Message { .. }) {
            return None;
        }

        let count = self.events[..=idx]
            .iter()
            .filter(|e| matches!(e, Event::Message { .. }))
            .count();
        Some(count)
    }

    pub fn add_box(&mut self, label: String, color: BoxColor, start: usize, end: usize) -> bool {
        if start >= self.participants.len() || end >= self.participants.len() || start > end {
            return false;
        }
        if self.box_overlaps(start, end) {
            return false;
        }
        self.boxes.push(ParticipantBox {
            label,
            color: Some(color),
            start,
            end,
        });
        self.boxes.sort_by_key(|b| b.start);
        true
    }

    pub fn overlapping_boxes(&self, start: usize, end: usize) -> Vec<usize> {
        (0..self.boxes.len())
            .filter(|&i| start <= self.boxes[i].end && self.boxes[i].start <= end)
            .collect()
    }

    pub fn box_overlaps(&self, start: usize, end: usize) -> bool {
        !self.overlapping_boxes(start, end).is_empty()
    }

    pub fn remove_box_at(&mut self, idx: usize) {
        if idx < self.boxes.len() {
            self.boxes.remove(idx);
        }
    }

    pub fn add_fragment(
        &mut self,
        kind: FragmentKind,
        label: String,
        start: usize,
        end: usize,
    ) -> bool {
        if start > end || end >= self.events.len() {
            return false;
        }
        if self.fragment_overlaps(start, end) {
            return false;
        }
        self.fragments.push(Fragment {
            kind,
            label,
            start,
            end,
            sections: Vec::new(),
        });
        self.fragments.sort_by_key(|f| f.start);
        true
    }

    pub fn overlapping_fragments(&self, start: usize, end: usize) -> Vec<usize> {
        (0..self.fragments.len())
            .filter(|&i| start <= self.fragments[i].end && self.fragments[i].start <= end)
            .collect()
    }

    pub fn fragment_overlaps(&self, start: usize, end: usize) -> bool {
        !self.overlapping_fragments(start, end).is_empty()
    }

    pub fn fragment_at(&self, event_idx: usize) -> Option<usize> {
        self.fragments
            .iter()
            .position(|f| f.start <= event_idx && event_idx <= f.end)
    }

    pub fn remove_fragment_at(&mut self, idx: usize) {
        if idx < self.fragments.len() {
            self.fragments.remove(idx);
        }
    }

    /// (frame index, divider index)
    pub fn section_at(&self, event_idx: usize) -> Option<(usize, usize)> {
        self.fragments.iter().enumerate().find_map(|(i, f)| {
            let k = f.sections.iter().position(|(at, _)| *at == event_idx)?;
            Some((i, k))
        })
    }

    pub fn rows_above(&self, idx: usize) -> u16 {
        let header = self.fragments.iter().any(|f| f.start == idx);
        u16::from(header || self.section_at(idx).is_some())
    }

    pub fn event_height(&self, idx: usize) -> u16 {
        let mut height = self.events.get(idx).map_or(3, Event::height);
        height += self.rows_above(idx);
        if self.fragments.iter().any(|f| f.end == idx) {
            height += 1;
        }
        height
    }

    pub fn box_index_of(&self, participant_idx: usize) -> Option<usize> {
        self.boxes
            .iter()
            .position(|b| b.start <= participant_idx && participant_idx <= b.end)
    }

    pub fn add_participant(&mut self, name: &str) {
        if is_valid_id(name) && self.index_of(name).is_none() {
            self.participants.push(Participant::new(name));
            return;
        }

        let mut participant = Participant::new(self.make_id(name));
        participant.label = Some(name.to_string());
        self.participants.push(participant);
    }

    fn index_of(&self, id: &str) -> Option<usize> {
        self.participants.iter().position(|p| p.id == id)
    }

    fn make_id(&self, name: &str) -> String {
        let valid: String = name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let base = if valid.is_empty() {
            "P".to_string()
        } else {
            valid
        };

        let mut id = base.clone();
        let mut n = 2;
        while self.index_of(&id).is_some() || is_reserved(&id) {
            id = format!("{base}{n}");
            n += 1;
        }
        id
    }

    pub fn rename_participant(&mut self, idx: usize, name: &str) {
        let Some(participant) = self.participants.get(idx) else {
            return;
        };

        if participant.label.is_some() {
            let label = (name != participant.id).then(|| name.to_string());
            self.participants[idx].label = label;
            return;
        }

        if is_valid_id(name) && self.index_of(name).is_none() {
            self.participants[idx].id = name.to_string();
        } else if name != participant.id {
            self.participants[idx].label = Some(name.to_string());
        }
    }

    pub fn toggle_actor(&mut self, idx: usize) -> Option<bool> {
        let participant = self.participants.get_mut(idx)?;
        participant.actor = !participant.actor;
        Some(participant.actor)
    }

    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// The new event joins the anchor's frame and section; without an anchor it is appended.
    pub fn insert_relative(&mut self, anchor: Option<usize>, before: bool, event: Event) -> usize {
        let (a, b) = event.endpoints();
        debug_assert!(a < self.participants.len() && b < self.participants.len());

        let Some(anchor) = anchor else {
            self.events.push(event);
            return self.events.len() - 1;
        };

        let at = (anchor + usize::from(!before)).min(self.events.len());
        let join = self.fragment_at(anchor);
        self.insert_event(at, event, join, !before);
        at
    }

    /// Inserts at the frame's first row: inside as its first event, or above its header.
    pub fn insert_at_frame(&mut self, frame_idx: usize, inside: bool, event: Event) -> usize {
        let at = self.fragments[frame_idx].start;
        self.insert_event(at, event, inside.then_some(frame_idx), false);
        at
    }

    /// A divider at `at` moves down with the events only if `shift_divider`.
    fn insert_event(&mut self, at: usize, event: Event, join: Option<usize>, shift_divider: bool) {
        let at = at.min(self.events.len());
        self.events.insert(at, event);

        for (i, f) in self.fragments.iter_mut().enumerate() {
            if Some(i) == join {
                f.end += 1;
                for (section_at, _) in &mut f.sections {
                    if *section_at > at || (*section_at == at && shift_divider) {
                        *section_at += 1;
                    }
                }
            } else if at <= f.start {
                f.move_to(f.start + 1);
            }
        }
    }

    pub fn remove_event(&mut self, idx: usize) {
        if idx >= self.events.len() {
            return;
        }
        self.events.remove(idx);

        self.fragments.retain(|f| f.start != idx || f.end != idx);
        for f in &mut self.fragments {
            if idx < f.start {
                f.move_to(f.start - 1);
            } else if idx <= f.end {
                f.end -= 1;
                for (at, _) in &mut f.sections {
                    if *at > idx {
                        *at -= 1;
                    }
                }
                f.drop_empty_sections();
            }
        }
    }

    /// At a frame edge the event crosses the border; at a divider the divider moves instead.
    pub fn move_event(&mut self, idx: usize, down: bool) -> usize {
        if idx >= self.events.len() {
            return idx;
        }
        if down {
            self.move_event_down(idx)
        } else {
            self.move_event_up(idx)
        }
    }

    fn move_event_down(&mut self, idx: usize) -> usize {
        if let Some(frame_idx) = self.fragment_at(idx)
            && self.fragments[frame_idx].end == idx
        {
            self.shrink_fragment(frame_idx, false);
            return idx;
        }
        if idx + 1 >= self.events.len() {
            return idx;
        }
        if let Some((frame_idx, k)) = self.section_at(idx + 1) {
            let f = &mut self.fragments[frame_idx];
            f.sections[k].0 -= 1;
            f.drop_empty_sections();
            return idx;
        }
        if let Some(f) = self.fragments.iter_mut().find(|f| f.start == idx + 1) {
            f.start -= 1;
            return idx;
        }
        self.events.swap(idx, idx + 1);
        idx + 1
    }

    fn move_event_up(&mut self, idx: usize) -> usize {
        if let Some(frame_idx) = self.fragment_at(idx)
            && self.fragments[frame_idx].start == idx
        {
            self.shrink_fragment(frame_idx, true);
            return idx;
        }
        if idx == 0 {
            return idx;
        }
        if let Some((frame_idx, k)) = self.section_at(idx) {
            let f = &mut self.fragments[frame_idx];
            f.sections[k].0 += 1;
            f.drop_empty_sections();
            return idx;
        }
        if let Some(f) = self.fragments.iter_mut().find(|f| f.end + 1 == idx) {
            f.end += 1;
            return idx;
        }
        self.events.swap(idx, idx - 1);
        idx - 1
    }

    pub fn move_fragment(&mut self, idx: usize, down: bool) -> usize {
        let Some(frame) = self.fragments.get(idx) else {
            return idx;
        };
        let (start, end) = (frame.start, frame.end);

        if !down {
            if start == 0 {
                return idx;
            }
            if idx > 0 && self.fragments[idx - 1].end + 1 == start {
                self.move_fragment(idx - 1, true);
                return idx - 1;
            }
            self.events[start - 1..=end].rotate_left(1);
            self.fragments[idx].move_to(start - 1);
            return idx;
        }

        if end + 1 >= self.events.len() {
            return idx;
        }
        if let Some(next) = self.fragments.get(idx + 1)
            && next.start == end + 1
        {
            let next_end = next.end;
            let next_len = next.end - next.start + 1;
            self.events[start..=next_end].rotate_left(end - start + 1);
            self.fragments[idx].move_to(start + next_len);
            self.fragments[idx + 1].move_to(start);
            self.fragments.swap(idx, idx + 1);
            return idx + 1;
        }
        self.events[start..=end + 1].rotate_right(1);
        self.fragments[idx].move_to(start + 1);
        idx
    }

    fn shrink_fragment(&mut self, frame_idx: usize, from_start: bool) {
        let f = &mut self.fragments[frame_idx];
        if f.start == f.end {
            self.fragments.remove(frame_idx);
            return;
        }
        if from_start {
            f.start += 1;
        } else {
            f.end -= 1;
        }
        f.drop_empty_sections();
    }

    /// None if the frame has no dividers or the spot is not valid.
    pub fn add_section(&mut self, frame_idx: usize, at: usize, label: String) -> Option<usize> {
        let f = self.fragments.get_mut(frame_idx)?;
        f.kind.divider()?;
        if at <= f.start || at > f.end {
            return None;
        }

        let k = f.sections.partition_point(|(s, _)| *s < at);
        if f.sections.get(k).is_some_and(|(s, _)| *s == at) {
            return None;
        }
        f.sections.insert(k, (at, label));
        Some(k)
    }

    /// Merges into the section above; the events stay.
    pub fn remove_section(&mut self, frame_idx: usize, k: usize) {
        if let Some(f) = self.fragments.get_mut(frame_idx)
            && k < f.sections.len()
        {
            f.sections.remove(k);
        }
    }

    /// Never empties a section.
    pub fn move_section(&mut self, frame_idx: usize, k: usize, down: bool) {
        let Some(f) = self.fragments.get_mut(frame_idx) else {
            return;
        };
        let Some(&(at, _)) = f.sections.get(k) else {
            return;
        };

        let above = if k == 0 { f.start } else { f.sections[k - 1].0 };
        let below = f.sections.get(k + 1).map_or(f.end + 1, |(s, _)| *s);

        if down && at + 1 < below {
            f.sections[k].0 += 1;
        } else if !down && at - 1 > above {
            f.sections[k].0 -= 1;
        }
    }

    pub fn swap_participants(&mut self, a: usize, b: usize) {
        if a >= self.participants.len() || b >= self.participants.len() {
            return;
        }
        self.participants.swap(a, b);
        for e in &mut self.events {
            match e {
                Event::Message { from, to, .. } => {
                    if *from == a {
                        *from = b;
                    } else if *from == b {
                        *from = a;
                    }
                    if *to == a {
                        *to = b;
                    } else if *to == b {
                        *to = a;
                    }
                }
                Event::Note {
                    participant_start,
                    participant_end,
                    ..
                } => {
                    if *participant_start == a {
                        *participant_start = b;
                    } else if *participant_start == b {
                        *participant_start = a;
                    }
                    if *participant_end == a {
                        *participant_end = b;
                    } else if *participant_end == b {
                        *participant_end = a;
                    }
                }
            }
        }
    }

    fn move_participant_block(&mut self, start: usize, end: usize, right: bool) {
        if right {
            for i in (start..=end).rev() {
                self.swap_participants(i, i + 1);
            }
        } else {
            for i in start..=end {
                self.swap_participants(i, i - 1);
            }
        }
    }

    pub fn move_box(&mut self, idx: usize, right: bool) -> usize {
        let Some(current) = self.boxes.get(idx) else {
            return idx;
        };
        let (start, end) = (current.start, current.end);

        if !right {
            if start == 0 {
                return idx;
            }
            if idx > 0 && self.boxes[idx - 1].end + 1 == start {
                self.move_box(idx - 1, true);
                return idx - 1;
            }
            self.move_participant_block(start, end, false);
            self.boxes[idx].start -= 1;
            self.boxes[idx].end -= 1;
            return idx;
        }

        if end + 1 >= self.participants.len() {
            return idx;
        }
        if let Some(next) = self.boxes.get(idx + 1)
            && next.start == end + 1
        {
            let next_len = next.end - next.start + 1;
            let own_len = end - start + 1;
            for step in 0..next_len {
                self.move_participant_block(start + step, end + step, true);
            }
            self.boxes[idx].start = start + next_len;
            self.boxes[idx].end = start + next_len + own_len - 1;
            self.boxes[idx + 1].start = start;
            self.boxes[idx + 1].end = start + next_len - 1;
            self.boxes.swap(idx, idx + 1);
            return idx + 1;
        }
        self.move_participant_block(start, end, true);
        self.boxes[idx].start += 1;
        self.boxes[idx].end += 1;
        idx
    }

    pub fn add_participant_to_box(&mut self, box_idx: usize, name: &str) {
        let Some(end) = self.boxes.get(box_idx).map(|b| b.end) else {
            return;
        };

        self.add_participant(name);
        let new_idx = self.participants.len() - 1;
        for i in (end + 1..new_idx).rev() {
            self.swap_participants(i, i + 1);
        }

        self.boxes[box_idx].end += 1;
        for later in &mut self.boxes[box_idx + 1..] {
            later.start += 1;
            later.end += 1;
        }
    }

    pub fn point_event_left(&mut self, idx: usize) {
        if let Some(Event::Message { from, to, .. }) = self.events.get_mut(idx)
            && *from < *to
        {
            std::mem::swap(from, to);
        }
    }

    pub fn point_event_right(&mut self, idx: usize) {
        if let Some(Event::Message { from, to, .. }) = self.events.get_mut(idx)
            && *from > *to
        {
            std::mem::swap(from, to);
        }
    }

    pub fn next_arrow_style(&mut self, idx: usize) -> Option<ArrowStyle> {
        if let Some(Event::Message { style, .. }) = self.events.get_mut(idx) {
            *style = style.next();
            return Some(*style);
        }
        None
    }

    pub fn prev_arrow_style(&mut self, idx: usize) -> Option<ArrowStyle> {
        if let Some(Event::Message { style, .. }) = self.events.get_mut(idx) {
            *style = style.prev();
            return Some(*style);
        }
        None
    }

    pub fn remove_participant(&mut self, idx: usize) {
        if idx >= self.participants.len() {
            return;
        }
        self.participants.remove(idx);
        for i in (0..self.events.len()).rev() {
            let (a, b) = self.events[i].endpoints();
            if a == idx || b == idx {
                self.remove_event(i);
            }
        }
        for e in &mut self.events {
            match e {
                Event::Message { from, to, .. } => {
                    if *from > idx {
                        *from -= 1;
                    }
                    if *to > idx {
                        *to -= 1;
                    }
                }
                Event::Note {
                    participant_start,
                    participant_end,
                    ..
                } => {
                    if *participant_start > idx {
                        *participant_start -= 1;
                    }
                    if *participant_end > idx {
                        *participant_end -= 1;
                    }
                }
            }
        }

        let mut new_boxes: Vec<ParticipantBox> = Vec::new();
        for mut b in self.boxes.drain(..) {
            if b.start == idx && b.end == idx {
                continue;
            }
            if b.end >= idx {
                b.end = b.end.saturating_sub(1);
            }
            if b.start > idx {
                b.start -= 1;
            }
            if b.start <= b.end {
                new_boxes.push(b);
            }
        }

        self.boxes = new_boxes;
    }

    pub fn to_mermaid(&self) -> String {
        let mut lines = vec!["sequenceDiagram".to_string()];
        if self.autonumber {
            lines.push("    autonumber".to_string());
        }

        let mut i = 0;
        while i < self.participants.len() {
            if let Some(b) = self.boxes.iter().find(|b| b.start == i) {
                let mut header = String::from("    box");
                if let Some(color) = b.color {
                    header.push(' ');
                    header.push_str(color.as_mermaid_str());
                }
                if !b.label.is_empty() {
                    header.push(' ');
                    header.push_str(&b.label);
                }
                lines.push(header);
                let box_end = b.end;
                for j in i..=box_end {
                    if let Some(p) = self.participants.get(j) {
                        lines.push(format!("        {}", declaration(p)));
                    }
                }
                lines.push("    end".to_string());
                i = box_end + 1;
            } else {
                if let Some(p) = self.participants.get(i) {
                    lines.push(format!("    {}", declaration(p)));
                }
                i += 1;
            }
        }

        for (i, event) in self.events.iter().enumerate() {
            if let Some(f) = self.fragments.iter().find(|f| f.start == i) {
                lines.push(format!("    {}", f.header()));
            }
            if let Some((frame, k)) = self.section_at(i)
                && let Some(line) = self.fragments[frame].divider_source(k)
            {
                lines.push(format!("    {line}"));
            }
            let first_line = lines.len();

            match event {
                Event::Message {
                    from,
                    to,
                    text,
                    style,
                } => {
                    if let (Some(from_p), Some(to_p)) =
                        (self.participants.get(*from), self.participants.get(*to))
                    {
                        let (from_name, to_name) = (&from_p.id, &to_p.id);
                        let arrow = style.as_mermaid_str();
                        lines.push(format!("    {from_name}{arrow}{to_name}: {text}"));
                    }
                }
                Event::Note {
                    position,
                    participant_start,
                    participant_end,
                    text,
                } => {
                    let pos_str = position.as_str();
                    if *position == NotePosition::Over && participant_start != participant_end {
                        if let (Some(start_p), Some(end_p)) = (
                            self.participants.get(*participant_start),
                            self.participants.get(*participant_end),
                        ) {
                            let (start_name, end_name) = (&start_p.id, &end_p.id);
                            lines.push(format!(
                                "    Note {pos_str} {start_name},{end_name}: {text}"
                            ));
                        }
                    } else if let Some(p) = self.participants.get(*participant_start) {
                        let name = &p.id;
                        lines.push(format!("    Note {pos_str} {name}: {text}"));
                    }
                }
            }

            if self.fragment_at(i).is_some() {
                for line in &mut lines[first_line..] {
                    line.insert_str(0, "    ");
                }
            }
            if self.fragments.iter().any(|f| f.end == i) {
                lines.push("    end".to_string());
            }
        }

        lines.join("\n") + "\n"
    }

    pub fn from_mermaid(input: &str) -> Result<Self> {
        let mut diagram = SequenceDiagram::new();
        let mut current_box: Option<(Option<BoxColor>, String, usize)> = None;
        let mut current_frame: Option<Fragment> = None;
        let mut seen_header = false;
        let mut open_line = 0;

        for (idx, line) in input.lines().enumerate() {
            let n = idx + 1;
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with("%%") {
                continue;
            }

            if !seen_header {
                if trimmed != "sequenceDiagram" {
                    bail!("line {n}: first line must be 'sequenceDiagram'");
                }
                seen_header = true;
                continue;
            }

            let had_open = current_box.is_some() || current_frame.is_some();
            diagram
                .parse_line(&mut current_box, &mut current_frame, trimmed)
                .map_err(|e| anyhow!("line {n}: {e}"))?;
            if !had_open && (current_box.is_some() || current_frame.is_some()) {
                open_line = n;
            }
        }

        if !seen_header {
            bail!("missing 'sequenceDiagram' header");
        }
        if current_box.is_some() {
            bail!("line {open_line}: 'box' is never closed");
        }
        if let Some(frame) = current_frame {
            bail!(
                "line {open_line}: '{}' is never closed",
                frame.kind.as_mermaid_str()
            );
        }

        Ok(diagram)
    }

    fn parse_line(
        &mut self,
        current_box: &mut Option<(Option<BoxColor>, String, usize)>,
        current_frame: &mut Option<Fragment>,
        trimmed: &str,
    ) -> Result<()> {
        if let Some((kind, label)) = FragmentKind::split_header(trimmed) {
            if current_box.is_some() {
                bail!("frames inside a box are not supported");
            }
            if current_frame.is_some() {
                bail!("nested frames are not supported yet");
            }
            *current_frame = Some(Fragment {
                kind,
                label,
                start: self.events.len(),
                end: 0,
                sections: Vec::new(),
            });
            return Ok(());
        }

        if let Some(first_word) = trimmed.split_whitespace().next() {
            if first_word == "autonumber" {
                return self
                    .parse_autonumber(trimmed, current_box.is_some() || current_frame.is_some());
            }
            if let Some(kind) = FragmentKind::with_divider(first_word) {
                return self.parse_divider(current_frame, kind, trimmed);
            }
            if BLOCK_KEYWORDS.contains(&first_word) {
                bail!("'{first_word}' blocks are not supported yet");
            }
            if OTHER_KEYWORDS.contains(&first_word) {
                bail!("'{first_word}' is not supported yet");
            }
        }

        if let Some(rest) = trimmed.strip_prefix("participant ") {
            return self.parse_declaration(rest, false, current_box.is_some(), trimmed);
        }

        if let Some(rest) = trimmed.strip_prefix("actor ") {
            return self.parse_declaration(rest, true, current_box.is_some(), trimmed);
        }

        if let Some(rest) = trimmed.strip_prefix("Note ") {
            let (position, after_pos) = if let Some(after) = rest.strip_prefix("right of ") {
                (NotePosition::Right, after)
            } else if let Some(after) = rest.strip_prefix("left of ") {
                (NotePosition::Left, after)
            } else if let Some(after) = rest.strip_prefix("over ") {
                (NotePosition::Over, after)
            } else {
                bail!("invalid note position: {trimmed}");
            };

            let Some(colon_pos) = after_pos.find(':') else {
                bail!("invalid note syntax (missing ':'): {trimmed}");
            };

            let participants_str = after_pos[..colon_pos].trim();
            let text = after_pos[colon_pos + 1..].trim().to_string();

            if position == NotePosition::Over && participants_str.contains(',') {
                let parts: Vec<&str> = participants_str.split(',').map(str::trim).collect();
                if parts.len() != 2 {
                    bail!("note over must have exactly 2 participants: {trimmed}");
                }
                let start_name = parts[0];
                let end_name = parts[1];

                let start_idx = self.index_of_or_insert(start_name)?;
                let end_idx = self.index_of_or_insert(end_name)?;

                self.events.push(Event::Note {
                    position,
                    participant_start: start_idx,
                    participant_end: end_idx,
                    text,
                });
            } else {
                let name = participants_str;
                if name.is_empty() {
                    bail!("invalid note syntax: {trimmed}");
                }

                let idx = self.index_of_or_insert(name)?;

                self.events.push(Event::Note {
                    position,
                    participant_start: idx,
                    participant_end: idx,
                    text,
                });
            }
            return Ok(());
        }

        if trimmed == "box" || trimmed.starts_with("box ") {
            if current_frame.is_some() {
                bail!("'box' inside a frame is not supported");
            }
            if current_box.is_some() {
                bail!("nested boxes are not supported");
            }
            let rest = trimmed.strip_prefix("box").unwrap().trim();
            let (color, label) = if rest.is_empty() {
                (None, String::new())
            } else {
                let (first_word, remainder) = rest
                    .split_once(|c: char| c.is_whitespace())
                    .map_or((rest, ""), |(f, r)| (f, r.trim()));
                if let Some(c) = BoxColor::from_mermaid_str(first_word) {
                    (Some(c), remainder.to_string())
                } else {
                    (None, rest.to_string())
                }
            };
            *current_box = Some((color, label, self.participants.len()));
            return Ok(());
        }

        if let Some((arrow_pos, style)) = find_arrow(trimmed) {
            let from_name = trimmed[..arrow_pos].trim();
            let rest = &trimmed[arrow_pos + style.as_mermaid_str().len()..];

            let (to_name, message) = if let Some(colon_pos) = rest.find(':') {
                let to = rest[..colon_pos].trim();
                let msg = rest[colon_pos + 1..].trim();
                (to, msg.to_string())
            } else {
                bail!("invalid message syntax (missing ':'): {trimmed}");
            };

            if to_name.starts_with(['+', '-']) {
                bail!("activation shorthand ('+'/'-') is not supported yet");
            }

            if from_name.is_empty() || to_name.is_empty() {
                bail!("invalid message syntax: {trimmed}");
            }

            if from_name.contains(['<', '>']) || to_name.contains(['<', '>']) {
                bail!("bidirectional arrows ('<<->>') are not supported yet");
            }

            let from_idx = self.index_of_or_insert(from_name)?;
            let to_idx = self.index_of_or_insert(to_name)?;

            self.events.push(Event::Message {
                from: from_idx,
                to: to_idx,
                text: message,
                style,
            });
            return Ok(());
        }

        if trimmed == "end" {
            if let Some((color, label, start)) = current_box.take() {
                let end = self.participants.len().saturating_sub(1);
                if end < start {
                    bail!("box has no participants");
                }
                self.boxes.push(ParticipantBox {
                    label,
                    color,
                    start,
                    end,
                });
            } else if let Some(mut frame) = current_frame.take() {
                if self.events.len() == frame.start {
                    bail!("'{}' block is empty", frame.kind.as_mermaid_str());
                }
                if self.events.len() == section_start(&frame) {
                    bail!("empty section before 'end'");
                }
                frame.end = self.events.len() - 1;
                self.fragments.push(frame);
            } else {
                bail!("unexpected 'end'");
            }
            return Ok(());
        }

        bail!("unsupported mermaid feature: {trimmed}");
    }

    fn parse_divider(
        &self,
        current_frame: &mut Option<Fragment>,
        kind: FragmentKind,
        trimmed: &str,
    ) -> Result<()> {
        let (word, label) = trimmed
            .split_once(char::is_whitespace)
            .unwrap_or((trimmed, ""));

        let frame = current_frame
            .as_mut()
            .filter(|f| f.kind == kind)
            .ok_or_else(|| {
                anyhow!(
                    "'{word}' is only allowed inside '{}'",
                    kind.as_mermaid_str()
                )
            })?;

        if self.events.len() == section_start(frame) {
            bail!("empty section before '{word}'");
        }
        frame
            .sections
            .push((self.events.len(), label.trim().to_string()));
        Ok(())
    }

    fn parse_autonumber(&mut self, trimmed: &str, nested: bool) -> Result<()> {
        if trimmed != "autonumber" {
            bail!("'autonumber' options are not supported yet");
        }
        if nested {
            bail!("'autonumber' is not supported inside a box or frame");
        }
        if !self.events.is_empty() {
            bail!("'autonumber' must come before the first message or note");
        }
        self.autonumber = true;
        Ok(())
    }

    fn index_of_or_insert(&mut self, id: &str) -> Result<usize> {
        if let Some(idx) = self.index_of(id) {
            return Ok(idx);
        }
        check_id(id)?;
        self.participants.push(Participant::new(id));
        Ok(self.participants.len() - 1)
    }

    fn parse_declaration(
        &mut self,
        rest: &str,
        actor: bool,
        in_box: bool,
        line: &str,
    ) -> Result<()> {
        if rest.trim() == "as" || rest.trim_end().ends_with(" as") {
            bail!("invalid participant declaration: {line}");
        }

        let (id, label) = match rest.split_once(" as ") {
            Some((id, label)) => (id.trim(), Some(label.trim())),
            None => (rest.trim(), None),
        };
        if id.is_empty() {
            bail!("invalid participant declaration: {line}");
        }
        if in_box && self.index_of(id).is_some() {
            bail!("box participants must be declared before use");
        }

        let idx = self.index_of_or_insert(id)?;
        let participant = &mut self.participants[idx];
        participant.actor = actor;
        if let Some(label) = label {
            participant.label = Some(label.to_string());
        }
        Ok(())
    }
}

fn section_start(frame: &Fragment) -> usize {
    frame.sections.last().map_or(frame.start, |(at, _)| *at)
}

fn declaration(p: &Participant) -> String {
    let keyword = if p.actor { "actor" } else { "participant" };
    match &p.label {
        Some(label) => format!("{keyword} {} as {label}", p.id),
        None => format!("{keyword} {}", p.id),
    }
}

fn is_valid_id(s: &str) -> bool {
    !s.is_empty() && !is_reserved(s) && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn is_reserved(s: &str) -> bool {
    BLOCK_KEYWORDS
        .iter()
        .chain(OTHER_KEYWORDS)
        .chain(RESERVED_WORDS)
        .any(|word| word.eq_ignore_ascii_case(s))
}

fn check_id(id: &str) -> Result<()> {
    let has_special = id.contains([',', ':', ';', '<', '>']);
    let starts_special = id.starts_with(['+', '-']) || id.starts_with("%%");
    let has_as = id.split_whitespace().any(|w| w == "as");
    let ends_dash = id.ends_with('-');
    let starts_keyword = id
        .split_once(char::is_whitespace)
        .is_some_and(|(first, _)| is_reserved(first));

    let invalid = has_special || starts_special || has_as || ends_dash || starts_keyword;
    if invalid || find_arrow(id).is_some() {
        bail!("unsupported participant id: {id}");
    }
    Ok(())
}

fn find_arrow(s: &str) -> Option<(usize, ArrowStyle)> {
    let before_colon = s.split(':').next().unwrap_or(s);

    if let Some(pos) = before_colon.find("->>")
        && !before_colon[..pos].ends_with('-')
    {
        return Some((pos, ArrowStyle::SolidArrow));
    }

    for (pos, _) in before_colon.match_indices('-') {
        for arrow in ARROWS {
            if before_colon[pos..].starts_with(arrow) {
                let style = ArrowStyle::from_mermaid_str(arrow).expect("ARROWS holds valid tokens");
                return Some((pos, style));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    impl SequenceDiagram {
        fn add_message(&mut self, from: usize, to: usize, text: String) {
            let style = ArrowStyle::default();
            let event = Event::Message {
                from,
                to,
                text,
                style,
            };
            self.insert_relative(None, false, event);
        }

        fn add_note(
            &mut self,
            position: NotePosition,
            participant_start: usize,
            participant_end: usize,
            text: String,
        ) {
            let event = Event::Note {
                position,
                participant_start,
                participant_end,
                text,
            };
            self.insert_relative(None, false, event);
        }
    }

    const MOVE_BOX_SOURCE: &str = "sequenceDiagram
    box Blue One
        participant A
        participant B
    end
    participant C
    box Two
        participant D
    end
    participant E
    A->>D: hi
    B->>C: yo
    D-->>A: back
    Note over B,E: n
    opt x
        C->>E: in
    end
";

    const FRAME_SOURCE: &str = "sequenceDiagram
    participant A
    participant B
    loop Every minute
        A->>B: ping
        Note over B: n
    end
    B->>A: pong
    opt
        A->>B: x
    end
";

    const ALT_SOURCE: &str = "sequenceDiagram
    participant A
    participant B
    A->>B: before
    alt ok
        A->>B: one
        A->>B: two
    else expired
        B->>A: three
    else
        B->>A: four
    end
    A->>B: after
";

    fn parse_err(input: &str) -> String {
        SequenceDiagram::from_mermaid(input)
            .err()
            .expect("expected parse error")
            .to_string()
    }

    fn ids(d: &SequenceDiagram) -> Vec<&str> {
        d.participants.iter().map(|p| p.id.as_str()).collect()
    }

    fn resolved(d: &SequenceDiagram) -> Vec<(String, String)> {
        let name = |i: usize| d.participants[i].id.clone();
        d.events
            .iter()
            .map(|e| {
                let (a, b) = e.endpoints();
                (name(a), name(b))
            })
            .collect()
    }

    fn box_ranges(d: &SequenceDiagram) -> Vec<(&str, usize, usize)> {
        d.boxes
            .iter()
            .map(|b| (b.label.as_str(), b.start, b.end))
            .collect()
    }

    fn assert_same_meaning(before: &SequenceDiagram, after: &SequenceDiagram) {
        assert_eq!(resolved(before), resolved(after));
        assert_eq!(before.fragments, after.fragments);
        let sorted = after.boxes.windows(2).all(|w| w[0].end < w[1].start);
        assert!(sorted);
        let reparsed = SequenceDiagram::from_mermaid(&after.to_mermaid()).unwrap();
        assert!(&reparsed == after);
    }

    fn first_style(d: &SequenceDiagram) -> ArrowStyle {
        match &d.events[0] {
            Event::Message { style, .. } => *style,
            Event::Note { .. } => panic!("Expected Message event"),
        }
    }

    fn framed() -> SequenceDiagram {
        SequenceDiagram::from_mermaid(FRAME_SOURCE).unwrap()
    }

    fn msg(d: &mut SequenceDiagram, text: &str) {
        d.add_message(0, 1, text.to_string());
    }

    fn frame_ranges(d: &SequenceDiagram) -> Vec<(usize, usize)> {
        d.fragments.iter().map(|f| (f.start, f.end)).collect()
    }

    fn numbered(n: usize) -> SequenceDiagram {
        let mut d = SequenceDiagram::new();
        d.add_participant("A");
        d.add_participant("B");
        for i in 0..n {
            msg(&mut d, &i.to_string());
        }
        d
    }

    fn texts(d: &SequenceDiagram) -> String {
        d.events.iter().map(Event::text).collect()
    }

    fn assert_reimports_equal(d: &SequenceDiagram) {
        let again = SequenceDiagram::from_mermaid(&d.to_mermaid()).unwrap();
        assert!(again == *d);
    }

    fn assert_sections_valid(d: &SequenceDiagram) {
        for f in &d.fragments {
            let mut previous = f.start;
            for (at, _) in &f.sections {
                assert!(*at > previous && *at <= f.end, "{:?}", f.sections);
                previous = *at;
            }
        }
    }

    fn sectioned(n: usize, start: usize, end: usize, at: &[usize]) -> SequenceDiagram {
        let mut d = numbered(n);
        assert!(d.add_fragment(FragmentKind::Alt, "f".into(), start, end));
        for &a in at {
            assert!(d.add_section(0, a, format!("s{a}")).is_some());
        }
        d
    }

    fn divider_starts(d: &SequenceDiagram) -> Vec<usize> {
        d.fragments[0].sections.iter().map(|(at, _)| *at).collect()
    }

    fn message_to(d: &SequenceDiagram, text: &str) -> Event {
        Event::Message {
            from: 0,
            to: 1,
            text: text.into(),
            style: d
                .events
                .iter()
                .find_map(|e| match e {
                    Event::Message { style, .. } => Some(*style),
                    Event::Note { .. } => None,
                })
                .unwrap_or_default(),
        }
    }

    fn simulate_app_edits(d: &mut SequenceDiagram) {
        for i in 0..d.participants.len() {
            let mut style = ArrowStyle::default();
            for _ in 0..8 {
                d.events.push(Event::Message {
                    from: i,
                    to: 0,
                    text: "m".to_string(),
                    style,
                });
                style = style.next();
            }
            d.events.push(Event::Note {
                position: NotePosition::Over,
                participant_start: i,
                participant_end: 0,
                text: "n".to_string(),
            });
        }
    }

    // parsing and exporting

    #[rstest]
    #[case::failure_missing_header_line(
        "Alice->>Bob: Hello",
        "line 1: first line must be 'sequenceDiagram'"
    )]
    #[case::failure_comment_only_input("%% only a comment\n", "missing 'sequenceDiagram' header")]
    #[case::failure_rect_block(
        "sequenceDiagram\nrect x",
        "line 2: 'rect' blocks are not supported yet"
    )]
    #[case::failure_create("sequenceDiagram\ncreate A", "line 2: 'create' is not supported yet")]
    #[case::failure_destroy("sequenceDiagram\ndestroy A", "line 2: 'destroy' is not supported yet")]
    #[case::failure_activate(
        "sequenceDiagram\nA->>B: x\nactivate B",
        "line 3: 'activate' is not supported yet"
    )]
    #[case::failure_deactivate(
        "sequenceDiagram\ndeactivate B",
        "line 2: 'deactivate' is not supported yet"
    )]
    #[case::failure_activation_shorthand(
        "sequenceDiagram\nA->>+B: x",
        "line 2: activation shorthand ('+'/'-') is not supported yet"
    )]
    #[case::failure_bidirectional_arrow(
        "sequenceDiagram\nA<<->>B: x",
        "line 2: bidirectional arrows ('<<->>') are not supported yet"
    )]
    #[case::failure_bidirectional_dashed_arrow(
        "sequenceDiagram\nA<<-->>B: x",
        "line 2: bidirectional arrows ('<<->>') are not supported yet"
    )]
    #[case::failure_box_without_participants(
        "sequenceDiagram\nparticipant A\nbox\nend",
        "line 4: box has no participants"
    )]
    #[case::failure_nested_boxes(
        "sequenceDiagram\nbox\nparticipant A\nbox\nend",
        "line 4: nested boxes are not supported"
    )]
    #[case::failure_autonumber_with_options(
        "sequenceDiagram\nautonumber 10",
        "line 2: 'autonumber' options are not supported yet"
    )]
    #[case::failure_autonumber_after_message(
        "sequenceDiagram\nA->>B: x\nautonumber",
        "line 3: 'autonumber' must come before the first message or note"
    )]
    #[case::failure_autonumber_in_box(
        "sequenceDiagram\nbox\nparticipant A\nautonumber",
        "line 4: 'autonumber' is not supported inside a box or frame"
    )]
    #[case::failure_autonumber_in_frame(
        "sequenceDiagram\nloop\nautonumber",
        "line 3: 'autonumber' is not supported inside a box or frame"
    )]
    // a block that is never closed is reported at its opening line
    #[case::failure_unclosed_loop(
        "sequenceDiagram\nparticipant A\nloop x\nA->>A: y",
        "line 3: 'loop' is never closed"
    )]
    #[case::failure_unclosed_box(
        "sequenceDiagram\nbox ->> A: x\nparticipant A\n",
        "line 2: 'box' is never closed"
    )]
    #[case::failure_nested_frames(
        "sequenceDiagram\nloop a\nopt b\nA->>B: x\nend\nend",
        "line 3: nested frames are not supported yet"
    )]
    #[case::failure_empty_loop("sequenceDiagram\nloop a\nend", "line 3: 'loop' block is empty")]
    #[case::failure_box_inside_frame(
        "sequenceDiagram\nloop a\nbox\nend",
        "line 3: 'box' inside a frame is not supported"
    )]
    #[case::failure_frame_inside_box(
        "sequenceDiagram\nbox\nloop a",
        "line 3: frames inside a box are not supported"
    )]
    #[case::failure_unexpected_end(
        "sequenceDiagram\nloop a\nA->>B: x\nend\nend",
        "line 5: unexpected 'end'"
    )]
    #[case::failure_else_outside_alt(
        "sequenceDiagram\nA->>B: x\nelse y",
        "line 3: 'else' is only allowed inside 'alt'"
    )]
    #[case::failure_else_in_par(
        "sequenceDiagram\npar\nA->>B: x\nelse y\nA->>B: z\nend",
        "line 4: 'else' is only allowed inside 'alt'"
    )]
    #[case::failure_and_in_alt(
        "sequenceDiagram\nalt\nA->>B: x\nand y\nA->>B: z\nend",
        "line 4: 'and' is only allowed inside 'par'"
    )]
    #[case::failure_option_in_par(
        "sequenceDiagram\npar\nA->>B: x\noption y\nA->>B: z\nend",
        "line 4: 'option' is only allowed inside 'critical'"
    )]
    #[case::failure_empty_section_between_elses(
        "sequenceDiagram\nalt a\nA->>B: x\nelse y\nelse z\nA->>B: x\nend",
        "line 5: empty section before 'else'"
    )]
    #[case::failure_empty_first_section_before_else(
        "sequenceDiagram\nalt a\nelse y\nA->>B: x\nend",
        "line 3: empty section before 'else'"
    )]
    #[case::failure_empty_last_section_before_end(
        "sequenceDiagram\nalt a\nA->>B: x\nelse y\nend",
        "line 5: empty section before 'end'"
    )]
    fn test_parse_error_message(#[case] given_input: &str, #[case] expected_error: &str) {
        assert_eq!(parse_err(given_input), expected_error);
    }

    #[rstest]
    #[case::failure_participant_id_with_cross_arrow("participant re-xport", "line 2:")]
    #[case::failure_participant_id_with_comma("participant a,b", "line 2:")]
    #[case::failure_actor_without_id("actor ->> B: hi", "line 2:")]
    #[case::failure_participant_with_empty_alias(
        "participant A as ",
        "line 2: invalid participant declaration"
    )]
    #[case::failure_actor_with_empty_id(
        "actor  as Alice",
        "line 2: invalid participant declaration"
    )]
    #[case::failure_sender_id_with_cross_arrow("data-xfer->>B: x", "line 2:")]
    #[case::failure_receiver_list("A->>b,c: x", "line 2:")]
    #[case::failure_receiver_with_cross_arrow("A->>B-xC: x", "line 2:")]
    #[case::failure_note_with_alias("Note over A as B: n", "line 2:")]
    #[case::failure_receiver_with_alias("A->>B as C: x", "line 2:")]
    #[case::failure_note_with_comment_marker("Note over %%x: n", "line 2:")]
    #[case::failure_note_with_trailing_dash("Note over a-: n", "line 2:")]
    #[case::failure_note_with_participant_keyword("Note over participant x: n", "line 2:")]
    fn test_unreadable_ids_are_rejected_with_line_number(
        #[case] given_body: &str,
        #[case] expected_prefix: &str,
    ) {
        let err = parse_err(&format!("sequenceDiagram\n{given_body}"));

        assert!(err.starts_with(expected_prefix), "{given_body}: {err}");
    }

    #[rstest]
    #[case::failure_participant_declared_before_box(
        "sequenceDiagram\nparticipant A\nbox\nparticipant A\nend"
    )]
    #[case::failure_participant_used_before_box(
        "sequenceDiagram\nA->>B: x\nbox\nparticipant B\nend"
    )]
    fn test_box_declaration_of_known_id_rejected(#[case] given_input: &str) {
        let err = parse_err(given_input);

        assert!(
            err.contains("box participants must be declared before use"),
            "{err}"
        );
    }

    #[rstest]
    #[case::success_box_with_aliases_and_note(
        "sequenceDiagram\nparticipant A as Alice\nbox Blue Team\nactor U as User\nparticipant B\nend\nU->>A: Hi\nNote over A,B: Hey\n",
        "sequenceDiagram\n    participant A as Alice\n    box Blue Team\n        actor U as User\n        participant B\n    end\n    U->>A: Hi\n    Note over A,B: Hey\n"
    )]
    #[case::success_implicit_participants(
        "sequenceDiagram\nA->>B: x",
        "sequenceDiagram\n    participant A\n    participant B\n    A->>B: x\n"
    )]
    #[case::success_comments_and_blank_lines_dropped(
        "%% intro\n\nsequenceDiagram\n\n%% note\n  participant Alice\nAlice->>Bob: Hi\n",
        "sequenceDiagram\n    participant Alice\n    participant Bob\n    Alice->>Bob: Hi\n"
    )]
    #[case::success_duplicate_autonumber_collapsed(
        "sequenceDiagram\nautonumber\nautonumber\nA->>B: hi",
        "sequenceDiagram\n    autonumber\n    participant A\n    participant B\n    A->>B: hi\n"
    )]
    fn test_normalizes_to(#[case] given_input: &str, #[case] expected_mermaid: &str) {
        let d = SequenceDiagram::from_mermaid(given_input).unwrap();

        assert_eq!(d.to_mermaid(), expected_mermaid);
    }

    #[rstest]
    #[case::success_frame(FRAME_SOURCE)]
    #[case::success_alt_frame(ALT_SOURCE)]
    #[case::success_box_and_loop(
        "sequenceDiagram\n    box Blue Team\n        participant A\n        participant B\n    end\n    loop L\n        A->>B: x\n    end\n"
    )]
    #[case::success_notes(
        "sequenceDiagram\n    participant A\n    participant B\n    Note right of A: r\n    Note left of B: l\n    Note over A,B: o\n"
    )]
    #[case::success_line_breaks(
        "sequenceDiagram\n    autonumber\n    participant A\n    participant B\n    A->>B: first line<br/>second line\n    B->>B: self<BR>loop\n    Note right of B: a<br />b<br/>c\n    loop retry\n        Note over A,B: framed<br>note\n    end\n"
    )]
    #[case::success_autonumber_box_and_frame(
        "sequenceDiagram\n    autonumber\n    box Blue Team\n        participant A\n    end\n    participant B\n    A->>B: hi\n    loop Retry\n        Note over A: n\n        B->>A: ok\n    end\n"
    )]
    // arrow tokens in labels must not be mistaken for message arrows
    #[case::success_arrow_in_else_label(
        "sequenceDiagram\n    participant A\n    participant B\n    alt a\n        A->>B: x\n    else a->>b\n        A->>B: y\n    end\n"
    )]
    #[case::success_arrow_in_loop_label_and_note(
        "sequenceDiagram\n    participant A\n    participant B\n    loop a->>b\n        A->>B: x\n        Note over A: see -x here\n    end\n"
    )]
    #[case::success_cross_arrow_in_box_label(
        "sequenceDiagram\n    box Blue a-xb\n        participant A\n    end\n"
    )]
    #[case::success_paren_in_box_label(
        "sequenceDiagram\n    box re-x)port\n        participant A\n    end\n"
    )]
    #[case::success_arrow_in_box_label(
        "sequenceDiagram\n    box a->>b\n        participant A\n    end\n"
    )]
    #[case::success_leading_arrow_in_box_label(
        "sequenceDiagram\n    box ->> A: x\n        participant A\n    end\n"
    )]
    fn test_exact_roundtrip(#[case] given_source: &str) {
        let d = SequenceDiagram::from_mermaid(given_source).unwrap();

        assert_eq!(d.to_mermaid(), given_source);
    }

    #[rstest]
    #[case::success_solid_arrow("->>", ArrowStyle::SolidArrow)]
    #[case::success_dashed_arrow("-->>", ArrowStyle::DashedArrow)]
    #[case::success_solid_line("->", ArrowStyle::Solid)]
    #[case::success_dashed_line("-->", ArrowStyle::Dashed)]
    #[case::success_solid_cross("-x", ArrowStyle::SolidCross)]
    #[case::success_dashed_cross("--x", ArrowStyle::DashedCross)]
    #[case::success_solid_async("-)", ArrowStyle::SolidAsync)]
    #[case::success_dashed_async("--)", ArrowStyle::DashedAsync)]
    fn test_arrow_style_parse_and_export(
        #[case] given_token: &str,
        #[case] expected_style: ArrowStyle,
    ) {
        let input = format!(
            "sequenceDiagram\n    participant A\n    participant B\n    A{given_token}B: Hi\n"
        );
        let d = SequenceDiagram::from_mermaid(&input).unwrap();

        assert!(matches!(d.events[0], Event::Message { from: 0, to: 1, .. }));
        assert_eq!(first_style(&d), expected_style);
        assert_eq!(d.to_mermaid(), input);
    }

    #[rstest]
    #[case::success_solid_arrow_with_arrow_text("A->>B: a -> b", ArrowStyle::SolidArrow)]
    #[case::success_dashed_arrow_with_arrow_text("A-->>B: a -> b", ArrowStyle::DashedArrow)]
    fn test_arrow_text_in_message_body(
        #[case] given_line: &str,
        #[case] expected_style: ArrowStyle,
    ) {
        let d = SequenceDiagram::from_mermaid(&format!("sequenceDiagram\n{given_line}")).unwrap();

        assert_eq!(d.events[0].text(), "a -> b");
        assert_eq!(first_style(&d), expected_style);
    }

    #[rstest]
    #[case::success_implicit_ids("Alice->>Bob: Hello", &["Alice", "Bob"])]
    #[case::success_id_with_dash("my-svc->>B: x", &["my-svc", "B"])]
    #[case::success_id_with_keyword_prefix("loop->>Bob: x", &["loop", "Bob"])]
    #[case::success_declared_ids_with_spaces("participant Actor\nparticipant Alice Smith", &["Actor", "Alice Smith"])]
    fn test_parsed_participant_ids(#[case] given_body: &str, #[case] expected_ids: &[&str]) {
        let d = SequenceDiagram::from_mermaid(&format!("sequenceDiagram\n{given_body}")).unwrap();

        assert_eq!(ids(&d), expected_ids);
    }

    #[test]
    fn test_parse_alias_and_actor() {
        let d = SequenceDiagram::from_mermaid(
            "sequenceDiagram\nparticipant A as Alice\nactor U as User\nactor Bob\nparticipant Carol\nU->>A: Hi",
        )
        .unwrap();

        assert_eq!(ids(&d), vec!["A", "U", "Bob", "Carol"]);
        assert_eq!(d.participants[0].label.as_deref(), Some("Alice"));
        assert!(!d.participants[0].actor);
        assert_eq!(d.participants[1].label.as_deref(), Some("User"));
        assert!(d.participants[1].actor);
        assert!(d.participants[2].actor);
        assert_eq!(d.participants[2].label, None);
        assert!(matches!(d.events[0], Event::Message { from: 1, to: 0, .. }));
    }

    #[test]
    fn test_declaration_after_use_updates_entry() {
        let d =
            SequenceDiagram::from_mermaid("sequenceDiagram\nA->>B: x\nactor A as Alice").unwrap();

        assert_eq!(ids(&d), vec!["A", "B"]);
        assert!(d.participants[0].actor);
        assert_eq!(d.participants[0].label.as_deref(), Some("Alice"));
    }

    #[rstest]
    #[case::success_note_right_of_participant(
        "Note right of A: hi",
        NotePosition::Right,
        0,
        0,
        "hi"
    )]
    #[case::success_note_left_of_participant("Note left of B: hi", NotePosition::Left, 1, 1, "hi")]
    #[case::success_note_over_participant_span(
        "Note over A,B: Spanning note",
        NotePosition::Over,
        0,
        1,
        "Spanning note"
    )]
    fn test_parse_note(
        #[case] given_line: &str,
        #[case] expected_position: NotePosition,
        #[case] expected_start: usize,
        #[case] expected_end: usize,
        #[case] expected_text: &str,
    ) {
        let source = format!("sequenceDiagram\nparticipant A\nparticipant B\n{given_line}");
        let d = SequenceDiagram::from_mermaid(&source).unwrap();

        let expected = Event::Note {
            position: expected_position,
            participant_start: expected_start,
            participant_end: expected_end,
            text: expected_text.to_string(),
        };
        assert!(d.events == [expected]);
    }

    #[test]
    fn test_valid_inputs_reimport_equal() {
        let inputs = [
            "sequenceDiagram\nparticipant A as Alice\nactor U as User\nU->>A: Hi",
            "sequenceDiagram\nparticipant Alice Smith\nparticipant my-svc\nAlice Smith->>my-svc: x",
            "sequenceDiagram\nbox Green Team\nactor A\nparticipant my-svc as Svc\nend\nA->>my-svc: x\nNote over A,my-svc: n\nNote left of A: l",
            "sequenceDiagram\nA->>B: x\nloop Every minute\nB->>A: y\nNote over A: n\nend\nopt\nA->>B: z\nend",
            "sequenceDiagram\nA->>B: 1\nA-->>B: 2\nA->B: 3\nA-->B: 4\nA-xB: 5\nA--xB: 6\nA-)B: 7\nA--)B: 8",
        ];
        for input in inputs {
            let mut first = SequenceDiagram::from_mermaid(input).unwrap();
            simulate_app_edits(&mut first);
            let second = SequenceDiagram::from_mermaid(&first.to_mermaid()).unwrap();
            assert!(first == second, "{input}");
        }
    }

    // arrow styles

    #[test]
    fn test_arrow_style_cycle() {
        let mut d = numbered(1);

        let start = first_style(&d);
        for _ in 0..8 {
            d.next_arrow_style(0);
        }
        assert_eq!(first_style(&d), start);

        d.next_arrow_style(0);
        d.prev_arrow_style(0);
        assert_eq!(first_style(&d), start);
    }

    #[test]
    fn test_arrow_style_ignores_non_messages() {
        let mut d = SequenceDiagram::new();
        d.add_participant("A");
        d.add_note(NotePosition::Over, 0, 0, "n".to_string());
        assert!(d.next_arrow_style(0).is_none());
        assert!(d.prev_arrow_style(5).is_none());
    }

    #[test]
    fn test_reverse_keeps_arrow_style() {
        let mut d = SequenceDiagram::from_mermaid("sequenceDiagram\nA-->>B: Hi").unwrap();
        d.point_event_left(0);
        assert_eq!(first_style(&d), ArrowStyle::DashedArrow);
    }

    // participants

    #[rstest]
    #[case::success_end("end")]
    #[case::success_box("Box")]
    #[case::success_else("else")]
    #[case::success_and("and")]
    #[case::success_option("option")]
    #[case::success_activate("activate")]
    #[case::success_autonumber("autonumber")]
    fn test_reserved_word_ids_get_suffix_and_label(#[case] given_name: &str) {
        let mut d = SequenceDiagram::new();
        d.add_participant(given_name);

        assert_eq!(d.participants[0].id, format!("{given_name}2"));
        assert_eq!(d.participants[0].label.as_deref(), Some(given_name));
        assert!(SequenceDiagram::from_mermaid(&d.to_mermaid()).is_ok());
    }

    #[test]
    fn test_add_participant_generates_ids() {
        let mut d = SequenceDiagram::new();
        d.add_participant("Alice Smith");
        d.add_participant("Bob");
        d.add_participant("Bob");
        d.add_participant(":::");

        assert_eq!(ids(&d), vec!["AliceSmith", "Bob", "Bob2", "P"]);
        assert_eq!(d.participants[0].label.as_deref(), Some("Alice Smith"));
        assert_eq!(d.participants[1].label, None);
        assert_eq!(d.participants[2].label.as_deref(), Some("Bob"));

        let parsed = SequenceDiagram::from_mermaid(&d.to_mermaid()).unwrap();
        assert_eq!(parsed.participants, d.participants);
    }

    #[test]
    fn test_rename_participant_rules() {
        let mut d = SequenceDiagram::new();
        d.add_participant("Alice");
        d.add_participant("Bob");

        d.rename_participant(0, "Carol");
        assert_eq!(d.participants[0], Participant::new("Carol"));

        d.rename_participant(0, "Bob");
        assert_eq!(d.participants[0].id, "Carol");
        assert_eq!(d.participants[0].label.as_deref(), Some("Bob"));

        d.rename_participant(0, "Carol");
        assert_eq!(d.participants[0], Participant::new("Carol"));

        d.rename_participant(1, "Big Bob");
        assert_eq!(d.participants[1].id, "Bob");
        assert_eq!(d.participants[1].label.as_deref(), Some("Big Bob"));

        d.rename_participant(1, "loop");
        assert_eq!(d.participants[1].id, "Bob");
        assert_eq!(d.participants[1].label.as_deref(), Some("loop"));
    }

    #[test]
    fn test_toggle_actor() {
        let mut d = SequenceDiagram::new();
        d.add_participant("Alice");

        assert_eq!(d.toggle_actor(0), Some(true));
        assert!(d.to_mermaid().contains("actor Alice"));
        assert_eq!(d.toggle_actor(0), Some(false));
        assert_eq!(d.toggle_actor(5), None);
    }

    #[test]
    fn test_swap_and_remove_keep_label_and_actor() {
        let mut d = SequenceDiagram::from_mermaid(
            "sequenceDiagram\nactor A as Alice\nparticipant B\nparticipant C as Carol",
        )
        .unwrap();

        d.swap_participants(0, 1);
        assert_eq!(ids(&d), vec!["B", "A", "C"]);
        assert!(d.participants[1].actor);
        assert_eq!(d.participants[1].label.as_deref(), Some("Alice"));

        d.remove_participant(0);
        assert_eq!(ids(&d), vec!["A", "C"]);
        assert!(d.participants[0].actor);
        assert_eq!(d.participants[1].label.as_deref(), Some("Carol"));
    }

    #[test]
    fn test_remove_participant_updates_frames() {
        let mut d = SequenceDiagram::from_mermaid(
            "sequenceDiagram\nA->>B: 1\nloop\nA->>C: 2\nB->>A: 3\nend\nB->>C: 4",
        )
        .unwrap();
        assert_eq!(frame_ranges(&d), vec![(1, 2)]);

        d.remove_participant(2);
        assert_eq!(frame_ranges(&d), vec![(1, 1)]);

        d.remove_participant(0);
        assert_eq!(d.fragments.len(), 0);
        assert_eq!(d.event_count(), 0);
    }

    #[test]
    fn test_remove_participant_with_sections() {
        let mut d = SequenceDiagram::from_mermaid(
            "sequenceDiagram\nalt a\nA->>B: 1\nelse b\nA->>C: 2\nelse c\nB->>A: 3\nend",
        )
        .unwrap();

        d.remove_participant(2);
        assert_eq!(divider_starts(&d), [1]);
        assert_eq!(d.fragments[0].sections[0].1, "c");
        assert_sections_valid(&d);

        d.remove_participant(1);
        assert_eq!(d.fragments.len(), 0);
    }

    // boxes

    #[test]
    fn test_box_roundtrip() {
        let mut diagram = SequenceDiagram::new();
        diagram.add_participant("Alice");
        diagram.add_participant("Bob");
        diagram.add_participant("Carol");
        diagram.add_box("Frontend".to_string(), BoxColor::Blue, 0, 1);
        diagram.add_message(0, 1, "Hello".to_string());

        let mermaid = diagram.to_mermaid();
        let parsed = SequenceDiagram::from_mermaid(&mermaid).unwrap();

        assert_eq!(parsed.participants, diagram.participants);
        assert_eq!(parsed.boxes, diagram.boxes);
    }

    #[rstest]
    #[case::success_plain_label("Foo", None, "Foo")]
    #[case::success_empty_header("", None, "")]
    #[case::success_rgb_header_stays_label("rgb(10, 20, 30) Foo", None, "rgb(10, 20, 30) Foo")]
    #[case::success_named_color("Red", Some(BoxColor::Red), "")]
    fn test_box_header_color_and_label(
        #[case] given_header: &str,
        #[case] expected_color: Option<BoxColor>,
        #[case] expected_label: &str,
    ) {
        let source =
            format!("sequenceDiagram\n    box {given_header}\n        participant A\n    end\n");
        let d = SequenceDiagram::from_mermaid(&source).unwrap();

        assert_eq!(d.boxes[0].color, expected_color);
        assert_eq!(d.boxes[0].label, expected_label);

        let line = format!("    box {given_header}");
        let exported = d.to_mermaid();
        assert!(exported.contains(&format!("{}\n", line.trim_end())));
        assert_reimports_equal(&d);
    }

    #[test]
    fn test_box_remove_participant() {
        let mut diagram = SequenceDiagram::new();
        diagram.add_participant("Alice");
        diagram.add_participant("Bob");
        diagram.add_participant("Carol");
        diagram.add_box("Group".to_string(), BoxColor::Green, 0, 2);

        diagram.remove_participant(0);
        assert_eq!(box_ranges(&diagram), vec![("Group", 0, 1)]);

        diagram.remove_participant(1);
        assert_eq!(box_ranges(&diagram), vec![("Group", 0, 0)]);

        diagram.remove_participant(0);
        assert!(diagram.boxes.is_empty(), "empty box should be dropped");
    }

    #[test]
    fn test_box_index_of() {
        let mut diagram = SequenceDiagram::new();
        diagram.add_participant("Alice");
        diagram.add_participant("Bob");
        diagram.add_participant("Carol");
        diagram.add_box("FE".to_string(), BoxColor::Blue, 0, 1);

        assert_eq!(diagram.box_index_of(0), Some(0));
        assert_eq!(diagram.box_index_of(1), Some(0));
        assert_eq!(diagram.box_index_of(2), None);
    }

    #[test]
    fn test_move_box_right_past_participant_and_back() {
        let original = SequenceDiagram::from_mermaid(MOVE_BOX_SOURCE).unwrap();
        let mut d = original.clone();

        assert_eq!(d.move_box(0, true), 0);
        assert_eq!(ids(&d), vec!["C", "A", "B", "D", "E"]);
        assert_eq!(box_ranges(&d), vec![("One", 1, 2), ("Two", 3, 3)]);
        assert_same_meaning(&original, &d);

        assert_eq!(d.move_box(0, false), 0);
        assert_eq!(ids(&d), vec!["A", "B", "C", "D", "E"]);
        assert!(d == original);
    }

    #[test]
    fn test_move_box_swaps_with_adjacent_box() {
        let original = SequenceDiagram::from_mermaid(MOVE_BOX_SOURCE).unwrap();
        let mut d = original.clone();
        d.move_box(0, true);

        assert_eq!(d.move_box(0, true), 1);
        assert_eq!(ids(&d), vec!["C", "D", "A", "B", "E"]);
        assert_eq!(box_ranges(&d), vec![("Two", 1, 1), ("One", 2, 3)]);
        assert_eq!(d.boxes[1].color, Some(BoxColor::Blue));
        assert_same_meaning(&original, &d);

        assert_eq!(d.move_box(1, false), 0);
        assert_eq!(ids(&d), vec!["C", "A", "B", "D", "E"]);
        assert_eq!(box_ranges(&d), vec![("One", 1, 2), ("Two", 3, 3)]);
        assert_same_meaning(&original, &d);
    }

    #[test]
    fn test_move_box_edges_are_noops() {
        let original = SequenceDiagram::from_mermaid(MOVE_BOX_SOURCE).unwrap();
        let mut d = original.clone();
        assert_eq!(d.move_box(0, false), 0);
        assert!(d == original);

        d.move_box(1, true);
        let at_end = d.clone();
        assert_eq!(d.move_box(1, true), 1);
        assert!(d == at_end);
    }

    #[test]
    fn test_add_participant_to_middle_box_shifts_later_boxes() {
        let original = SequenceDiagram::from_mermaid(MOVE_BOX_SOURCE).unwrap();
        let mut d = original.clone();

        d.add_participant_to_box(0, "Z");

        assert_eq!(ids(&d), vec!["A", "B", "Z", "C", "D", "E"]);
        assert_eq!(box_ranges(&d), vec![("One", 0, 2), ("Two", 4, 4)]);
        assert_eq!(resolved(&d), resolved(&original));
        assert_reimports_equal(&d);
    }

    // frames and sections

    #[test]
    fn test_frame_parse_all_kinds_with_and_without_label() {
        for kind in FragmentKind::ALL {
            let name = kind.as_mermaid_str();
            let with = format!("sequenceDiagram\n{name} Some label\nA->>B: x\nend");
            let d = SequenceDiagram::from_mermaid(&with).unwrap();
            assert_eq!(d.fragments.len(), 1);
            assert_eq!(d.fragments[0].kind, kind);
            assert_eq!(d.fragments[0].label, "Some label");

            let without = format!("sequenceDiagram\n{name}\nA->>B: x\nend");
            let d = SequenceDiagram::from_mermaid(&without).unwrap();
            assert_eq!(d.fragments[0].label, "");
        }
    }

    #[test]
    fn test_framed_diagram_ranges_and_heights() {
        let d = framed();

        assert_eq!(frame_ranges(&d), vec![(0, 1), (3, 3)]);
        assert_eq!(d.event_height(0), 4);
        assert_eq!(d.event_height(1), 3);
        assert_eq!(d.event_height(2), 3);
        assert_eq!(d.event_height(3), 5);
    }

    #[test]
    fn test_multiline_event_heights() {
        let d = SequenceDiagram::from_mermaid(
            "sequenceDiagram\nloop\nA->>B: x<br/>y\nend\nNote right of B: a<br>b<br>c\n",
        )
        .unwrap();
        assert_eq!(d.event_height(0), 6);
        assert_eq!(d.event_height(1), 4);
    }

    #[test]
    fn test_alt_sections_parse() {
        let d = SequenceDiagram::from_mermaid(ALT_SOURCE).unwrap();
        let f = &d.fragments[0];

        assert_eq!(f.kind, FragmentKind::Alt);
        assert_eq!((f.start, f.end), (1, 4));
        assert_eq!(f.label, "ok");
        assert_eq!(f.sections, [(3, "expired".to_string()), (4, String::new())]);
        assert_sections_valid(&d);
    }

    #[test]
    fn test_every_sectioned_kind_parses_and_roundtrips() {
        for kind in FragmentKind::ALL {
            let Some(word) = kind.divider() else {
                continue;
            };
            let keyword = kind.as_mermaid_str();
            for (head, label) in [("a", "b"), ("", "")] {
                let source = format!(
                    "sequenceDiagram\n    participant A\n    participant B\n    {keyword} {head}\n        A->>B: x\n    {word} {label}\n        A->>B: y\n    {word}\n        A->>B: z\n    end\n"
                );
                let source = source.replace(" \n", "\n");

                let d = SequenceDiagram::from_mermaid(&source).unwrap();
                assert_eq!(d.fragments[0].kind, kind);
                assert_eq!(d.fragments[0].sections.len(), 2);
                assert_eq!(d.fragments[0].sections[0].1, label);
                assert_sections_valid(&d);
                assert_eq!(d.to_mermaid(), source, "{keyword}");
            }
        }
    }

    #[test]
    fn test_add_section_requires_a_kind_with_dividers() {
        for kind in FragmentKind::ALL {
            let mut d = numbered(3);
            d.add_fragment(kind, String::new(), 0, 2);
            assert_eq!(
                d.add_section(0, 1, String::new()).is_some(),
                kind.divider().is_some()
            );
        }
    }

    #[test]
    fn test_section_at_rows_above_and_event_height() {
        let d = SequenceDiagram::from_mermaid(ALT_SOURCE).unwrap();

        assert_eq!(d.section_at(3), Some((0, 0)));
        assert_eq!(d.section_at(4), Some((0, 1)));
        assert_eq!(d.section_at(2), None);
        assert_eq!(d.rows_above(1), 1);
        assert_eq!(d.rows_above(3), 1);
        assert_eq!(d.rows_above(2), 0);
        assert_eq!(d.event_height(3), d.events[3].height() + 1);
        assert_eq!(d.event_height(4), d.events[4].height() + 2);
        assert_eq!(d.event_height(2), d.events[2].height());
    }

    #[test]
    fn test_add_fragment_rejects_overlap_and_bad_ranges() {
        let mut d = numbered(4);

        assert!(d.add_fragment(FragmentKind::Loop, String::new(), 1, 2));
        assert!(!d.add_fragment(FragmentKind::Opt, String::new(), 2, 3));
        assert!(!d.add_fragment(FragmentKind::Opt, String::new(), 0, 1));
        assert!(!d.add_fragment(FragmentKind::Opt, String::new(), 3, 4));
        assert!(!d.add_fragment(FragmentKind::Opt, String::new(), 3, 2));
        assert!(d.add_fragment(FragmentKind::Opt, String::new(), 3, 3));
        assert_eq!(frame_ranges(&d), vec![(1, 2), (3, 3)]);
    }

    #[test]
    fn test_add_section_rules() {
        let mut d = numbered(5);
        d.add_fragment(FragmentKind::Alt, "f".into(), 1, 3);
        d.add_fragment(FragmentKind::Loop, "l".into(), 4, 4);

        assert_eq!(d.add_section(0, 2, "b".into()), Some(0));
        assert_eq!(d.add_section(0, 3, "c".into()), Some(1));
        assert_eq!(d.add_section(0, 2, "x".into()), None);
        assert_eq!(d.add_section(0, 1, "x".into()), None);
        assert_eq!(d.add_section(0, 4, "x".into()), None);
        assert_eq!(d.add_section(1, 4, "x".into()), None);
        assert_eq!(d.add_section(7, 4, "x".into()), None);
        assert_eq!(divider_starts(&d), [2, 3]);

        let mut d = numbered(5);
        d.add_fragment(FragmentKind::Alt, "f".into(), 0, 4);
        d.add_section(0, 3, "late".into());
        assert_eq!(d.add_section(0, 1, "early".into()), Some(0));
        assert_eq!(divider_starts(&d), [1, 3]);
        assert_sections_valid(&d);
    }

    #[test]
    fn test_remove_section_merges_into_section_above() {
        let mut d = sectioned(5, 0, 4, &[2, 3]);
        d.remove_section(0, 0);

        assert_eq!(divider_starts(&d), [3]);
        assert_eq!(d.event_count(), 5);
        d.remove_section(0, 5);
        assert_eq!(divider_starts(&d), [3]);
    }

    #[test]
    fn test_move_section_limits() {
        let mut d = sectioned(6, 0, 5, &[2, 4]);
        let step = |d: &mut SequenceDiagram, k: usize, down: bool| {
            d.move_section(0, k, down);
            divider_starts(d)
        };

        assert_eq!(step(&mut d, 0, true), [3, 4]);
        assert_eq!(step(&mut d, 0, true), [3, 4]);
        assert_eq!(step(&mut d, 1, true), [3, 5]);
        assert_eq!(step(&mut d, 1, true), [3, 5]);
        assert_eq!(step(&mut d, 0, false), [2, 5]);
        assert_eq!(step(&mut d, 0, false), [1, 5]);
        assert_eq!(step(&mut d, 0, false), [1, 5]);
        assert_sections_valid(&d);
    }

    // inserting events

    #[rstest]
    #[case::success_before_first_framed_event_joins_frame(Some(0), true, 0, vec![(0, 2), (4, 4)])]
    #[case::success_after_last_framed_event_joins_frame(Some(1), false, 2, vec![(0, 2), (4, 4)])]
    #[case::success_before_free_event_stays_outside_frame(Some(2), true, 2, vec![(0, 1), (4, 4)])]
    #[case::success_after_free_event_stays_outside_frame(Some(2), false, 3, vec![(0, 1), (4, 4)])]
    #[case::success_before_event_of_second_frame_joins_it(Some(3), true, 3, vec![(0, 1), (3, 4)])]
    #[case::success_no_anchor_appends_outside_frames(None, false, 4, vec![(0, 1), (3, 3)])]
    fn test_insert_relative_joins_frame_of_anchor_only(
        #[case] given_anchor: Option<usize>,
        #[case] given_before: bool,
        #[case] expected_idx: usize,
        #[case] expected_frames: Vec<(usize, usize)>,
    ) {
        let mut d = framed();
        let event = message_to(&d, "new");

        let idx = d.insert_relative(given_anchor, given_before, event);

        assert_eq!(idx, expected_idx);
        assert_eq!(frame_ranges(&d), expected_frames);
    }

    #[rstest]
    #[case::success_after_last_event_of_section(sectioned(4, 0, 3, &[2]), Some(1), false, 2, vec![3], (0, 4))]
    #[case::success_before_first_event_of_next_section(sectioned(4, 0, 3, &[2]), Some(2), true, 2, vec![2], (0, 4))]
    #[case::success_after_first_event_before_divider(sectioned(4, 0, 3, &[2]), Some(0), false, 1, vec![3], (0, 4))]
    #[case::success_before_first_framed_event(sectioned(5, 1, 4, &[3]), Some(1), true, 1, vec![4], (1, 5))]
    #[case::success_after_event_before_frame(sectioned(5, 1, 4, &[3]), Some(0), false, 1, vec![4], (2, 5))]
    fn test_insert_next_to_divider_lands_in_anchor_section(
        #[case] mut given_diagram: SequenceDiagram,
        #[case] given_anchor: Option<usize>,
        #[case] given_before: bool,
        #[case] expected_idx: usize,
        #[case] expected_starts: Vec<usize>,
        #[case] expected_frame: (usize, usize),
    ) {
        let event = message_to(&given_diagram, "n");

        let idx = given_diagram.insert_relative(given_anchor, given_before, event);

        assert_eq!(idx, expected_idx);
        assert_eq!(divider_starts(&given_diagram), expected_starts);
        assert_eq!(frame_ranges(&given_diagram), [expected_frame]);
        assert_sections_valid(&given_diagram);
        assert_reimports_equal(&given_diagram);
    }

    #[test]
    fn test_insert_at_frame_inside_vs_before() {
        let mut d = numbered(2);
        d.add_fragment(FragmentKind::Opt, String::new(), 1, 1);

        assert_eq!(d.insert_at_frame(0, true, message_to(&d, "in")), 1);
        assert_eq!(texts(&d), "0in1");
        assert_eq!(frame_ranges(&d), vec![(1, 2)]);

        assert_eq!(d.insert_at_frame(0, false, message_to(&d, "out")), 1);
        assert_eq!(texts(&d), "0outin1");
        assert_eq!(frame_ranges(&d), vec![(2, 3)]);
    }

    #[test]
    fn test_insert_at_frame_inside_shifts_dividers() {
        let mut d = sectioned(5, 1, 4, &[3]);
        let new = message_to(&d, "n");

        d.insert_at_frame(0, true, new);

        assert_eq!(divider_starts(&d), [4]);
        assert_sections_valid(&d);
    }

    // removing events

    #[test]
    fn test_remove_event_shrinks_and_drops_frames() {
        let mut d = framed();
        d.remove_event(0);
        assert_eq!(frame_ranges(&d), vec![(0, 0), (2, 2)]);

        d.remove_event(0);
        assert_eq!(frame_ranges(&d), vec![(1, 1)]);

        d.remove_event(1);
        assert_eq!(d.fragments.len(), 0);
    }

    #[rstest]
    #[case::success_remove_event_in_first_section(vec![2], vec![2, 4], (1, 5))]
    #[case::success_remove_first_event_of_section(vec![3], vec![3, 4], (1, 5))]
    #[case::success_remove_last_events_drops_divider(vec![6, 5], vec![3], (1, 4))]
    #[case::success_remove_event_before_frame(vec![0], vec![2, 4], (0, 5))]
    fn test_remove_event_in_sections(
        #[case] given_removed: Vec<usize>,
        #[case] expected_starts: Vec<usize>,
        #[case] expected_frame: (usize, usize),
    ) {
        let mut d = sectioned(7, 1, 6, &[3, 5]);

        for idx in given_removed {
            d.remove_event(idx);
        }

        assert_eq!(divider_starts(&d), expected_starts);
        assert_eq!(frame_ranges(&d), [expected_frame]);
        assert_sections_valid(&d);
    }

    #[test]
    fn test_remove_first_section_takes_label_of_first_divider() {
        let mut d = sectioned(5, 0, 4, &[2]);
        d.remove_event(0);
        d.remove_event(0);

        assert_eq!(divider_starts(&d), Vec::<usize>::new());
        assert_eq!(d.fragments[0].label, "s2");
        assert_eq!((d.fragments[0].start, d.fragments[0].end), (0, 2));
        assert_reimports_equal(&d);
    }

    // moving events

    #[test]
    fn test_move_event_leaves_and_enters_frames() {
        let mut d = framed();

        // J on the last framed event leaves the frame without reordering
        assert_eq!(d.move_event(1, true), 1);
        assert_eq!(frame_ranges(&d), vec![(0, 0), (3, 3)]);

        assert_eq!(d.move_event(1, true), 2);
        assert_eq!(frame_ranges(&d), vec![(0, 0), (3, 3)]);

        assert_eq!(d.move_event(2, true), 2);
        assert_eq!(frame_ranges(&d), vec![(0, 0), (2, 3)]);

        assert_eq!(d.move_event(2, false), 2);
        assert_eq!(frame_ranges(&d), vec![(0, 0), (3, 3)]);

        assert_eq!(d.move_event(1, false), 1);
        assert_eq!(frame_ranges(&d), vec![(0, 1), (3, 3)]);
    }

    #[test]
    fn test_move_event_swaps_inside_frame_and_removes_single_event_frame() {
        let mut d = framed();
        assert_eq!(d.move_event(0, true), 1);
        assert_eq!(frame_ranges(&d), vec![(0, 1), (3, 3)]);

        assert_eq!(d.move_event(3, false), 3);
        assert_eq!(frame_ranges(&d), vec![(0, 1)]);
    }

    #[test]
    fn test_move_event_across_divider_moves_the_divider() {
        let mut d = sectioned(6, 0, 5, &[3]);

        assert_eq!(d.move_event(2, true), 2);
        assert_eq!(divider_starts(&d), [2]);
        assert_eq!(texts(&d), "012345");

        assert_eq!(d.move_event(2, false), 2);
        assert_eq!(divider_starts(&d), [3]);
        assert_sections_valid(&d);
    }

    #[rstest]
    #[case::success_move_down_from_single_event_section(&[1, 2], 1, false, vec![2])]
    #[case::success_move_last_event_out_of_frame(&[1, 3], 3, true, vec![1])]
    #[case::success_move_up_from_single_event_section(&[1, 2], 1, true, vec![1])]
    fn test_move_event_that_empties_a_section_drops_the_divider(
        #[case] given_at: &[usize],
        #[case] given_idx: usize,
        #[case] given_down: bool,
        #[case] expected_starts: Vec<usize>,
    ) {
        let mut d = sectioned(4, 0, 3, given_at);

        d.move_event(given_idx, given_down);

        assert_eq!(divider_starts(&d), expected_starts);
        assert_sections_valid(&d);
        assert_reimports_equal(&d);
    }

    #[test]
    fn test_shrinking_frame_drops_empty_sections() {
        let mut d = sectioned(4, 0, 3, &[1, 3]);
        d.move_event(0, false);
        assert_eq!((d.fragments[0].start, d.fragments[0].end), (1, 3));
        assert_eq!(divider_starts(&d), [3]);
        assert_eq!(d.fragments[0].label, "s1");
        assert_sections_valid(&d);
    }

    // moving frames

    #[test]
    fn test_move_fragment_moves_dividers_along() {
        let mut d = sectioned(6, 1, 3, &[2]);

        assert_eq!(d.move_fragment(0, true), 0);
        assert_eq!(divider_starts(&d), [3]);
        assert_eq!(d.move_fragment(0, false), 0);
        assert_eq!(d.move_fragment(0, false), 0);
        assert_eq!(divider_starts(&d), [1]);
        assert_sections_valid(&d);
        assert_reimports_equal(&d);
    }

    #[test]
    fn test_move_fragment_past_event_and_back() {
        let mut d = numbered(4);
        d.add_fragment(FragmentKind::Opt, "x".into(), 1, 2);

        assert_eq!(d.move_fragment(0, true), 0);
        assert_eq!(texts(&d), "0312");
        assert_eq!(frame_ranges(&d), vec![(2, 3)]);
        assert_reimports_equal(&d);

        assert_eq!(d.move_fragment(0, true), 0);
        assert_eq!(texts(&d), "0312");

        assert_eq!(d.move_fragment(0, false), 0);
        assert_eq!(d.move_fragment(0, false), 0);
        assert_eq!(texts(&d), "1203");
        assert_eq!(frame_ranges(&d), vec![(0, 1)]);
        assert_eq!(d.move_fragment(0, false), 0);
        assert_reimports_equal(&d);
    }

    #[test]
    fn test_move_fragment_swaps_with_adjacent_frame_and_moves_dividers() {
        let mut d = numbered(6);
        d.add_fragment(FragmentKind::Alt, "a".into(), 0, 1);
        d.add_section(0, 1, "x".into());
        d.add_fragment(FragmentKind::Alt, "b".into(), 2, 5);
        d.add_section(1, 4, "y".into());

        assert_eq!(d.move_fragment(0, true), 1);
        assert_eq!(texts(&d), "234501");
        assert_eq!(frame_ranges(&d), vec![(0, 3), (4, 5)]);
        assert_eq!(d.fragments[0].label, "b");
        assert_eq!(d.fragments[0].sections[0].0, 2);
        assert_eq!(d.fragments[1].sections[0].0, 5);
        assert_reimports_equal(&d);

        assert_eq!(d.move_fragment(1, false), 0);
        assert_eq!(texts(&d), "012345");
        assert_eq!(d.fragments[0].label, "a");
        assert_eq!(d.fragments[0].sections[0].0, 1);
        assert_eq!(d.fragments[1].sections[0].0, 4);
        assert_reimports_equal(&d);
    }

    #[test]
    fn test_move_fragment_is_noop_at_edges() {
        let mut d = numbered(3);
        d.add_fragment(FragmentKind::Opt, String::new(), 0, 2);
        let before = d.clone();
        assert_eq!(d.move_fragment(0, true), 0);
        assert_eq!(d.move_fragment(0, false), 0);
        assert!(d == before);
    }

    // autonumber

    #[test]
    fn test_message_number() {
        let mut d = SequenceDiagram::from_mermaid(
            "sequenceDiagram\nautonumber\nA->>B: one\nNote over A: n\nA->>A: two\nB->>A: three",
        )
        .unwrap();
        let numbers: Vec<_> = (0..4).map(|i| d.message_number(i)).collect();
        assert_eq!(numbers, vec![Some(1), None, Some(2), Some(3)]);

        assert!(!d.toggle_autonumber());
        assert!((0..4).all(|i| d.message_number(i).is_none()));
    }
}
