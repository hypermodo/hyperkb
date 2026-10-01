use crate::core::MetadataParser;
use crate::domain::{Document, DocumentStatus};
use crate::ui::theme::Theme;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub struct MarkdownFormatter;

#[derive(Clone, Debug)]
struct StyledWord {
    text: String,
    style: Style,
}

impl MarkdownFormatter {
    /// Formats document metadata into a clean, structured top card.
    pub fn format_metadata_card(doc: &Document) -> Vec<Line<'static>> {
        let (badge_text, badge_style) = match doc.status {
            DocumentStatus::Accepted => ("● ACCEPTED", Theme::badge_accepted()),
            DocumentStatus::Proposed => ("○ PROPOSED", Theme::badge_proposed()),
            DocumentStatus::Open => ("▲ OPEN", Theme::badge_risk()),
            DocumentStatus::Acknowledged => ("✔ ACKNOWLEDGED", Style::default().fg(Color::Yellow)),
            DocumentStatus::Resolved => ("✔ RESOLVED", Theme::badge_accepted()),
            DocumentStatus::Superseded => ("✕ SUPERSEDED", Style::default().fg(Theme::STATUS_SUPERSEDED)),
            DocumentStatus::Conflict => ("! CONFLICT", Style::default().fg(Color::LightRed)),
            DocumentStatus::Unknown => ("· UNKNOWN", Style::default().fg(Theme::STATUS_UNKNOWN)),
        };

        let owner_str = if doc.owner.is_empty() {
            "None".to_string()
        } else {
            doc.owner.clone()
        };
        let id_str = if doc.id.len() > 16 {
            doc.id[..16].to_string()
        } else {
            doc.id.clone()
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled("  Status: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(badge_text, badge_style),
                Span::raw("    "),
                Span::styled("Kind: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(
                    doc.kind.as_str().to_uppercase(),
                    Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD),
                ),
                Span::raw("    "),
                Span::styled("Owner: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(owner_str, Style::default().fg(Color::White)),
                Span::raw("    "),
                Span::styled("Topic: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(doc.topic.clone(), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("  Source: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(doc.path.clone(), Style::default().fg(Theme::ACCENT)),
                Span::raw("    "),
                Span::styled("ID: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(id_str, Style::default().fg(Color::DarkGray)),
            ]),
        ];

        if let Some(ref supersedes) = doc.supersedes {
            lines.push(Line::from(vec![
                Span::styled("  Supersedes: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(supersedes.clone(), Style::default().fg(Color::Yellow)),
            ]));
        }

        if let Some(ref repl) = doc.replacement_id {
            lines.push(Line::from(vec![
                Span::styled("  Replacement: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(repl.clone(), Style::default().fg(Theme::STATUS_ACCEPTED)),
            ]));
        }

        lines
    }

    /// Formats the Markdown body into styled Ratatui Lines with explicit width wrapping and hanging indents.
    pub fn format_markdown(content: &str, max_width: usize) -> Vec<Line<'static>> {
        let max_width = max_width.max(40);
        let content_width = max_width.saturating_sub(2).max(25);
        let parsed = MetadataParser::parse(content).unwrap_or_else(|_| crate::core::ParsedMetadata {
            meta: None,
            body: content,
            title: "Document".into(),
        });

        let mut lines = Vec::new();
        let mut in_code_block = false;
        let mut raw_lines = parsed.body.lines().peekable();

        while let Some(line) = raw_lines.next() {
            let trimmed = line.trim();

            // 1. Fenced Code Blocks (```rust, ```bash, etc.)
            if trimmed.starts_with("```") {
                let code_width = max_width.min(84).max(45);
                if in_code_block {
                    in_code_block = false;
                    let bar_len = code_width.saturating_sub(6);
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(
                            format!("└{:─<bar_len$}┘", "", bar_len = bar_len),
                            Style::default().fg(Color::Rgb(70, 70, 85)),
                        ),
                    ]));
                } else {
                    in_code_block = true;
                    let code_lang = trimmed.trim_start_matches("```").trim();
                    let lang_tag = if code_lang.is_empty() {
                        "".to_string()
                    } else {
                        format!(" [ {} ]", code_lang)
                    };
                    let bar_len = code_width.saturating_sub(13 + lang_tag.chars().count());
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(
                            format!("┌── Code{} {:─<bar_len$}┐", lang_tag, "", bar_len = bar_len),
                            Style::default().fg(Color::Rgb(70, 70, 85)),
                        ),
                    ]));
                }
                continue;
            }

            if in_code_block {
                let code_width = max_width.min(84).max(45);
                let inner_code = code_width.saturating_sub(8);
                let code_len = line.chars().count();
                let display_code = if code_len > inner_code {
                    let mut truncated: String = line.chars().take(inner_code.saturating_sub(1)).collect();
                    truncated.push('…');
                    truncated
                } else {
                    line.to_string()
                };
                let pad_len = inner_code.saturating_sub(display_code.chars().count());
                lines.push(Line::from(vec![
                    Span::styled("    │ ", Style::default().fg(Color::Rgb(70, 70, 85))),
                    Span::styled(display_code, Style::default().fg(Color::Rgb(147, 197, 253))),
                    Span::raw(" ".repeat(pad_len)),
                    Span::styled(" │", Style::default().fg(Color::Rgb(70, 70, 85))),
                ]));
                continue;
            }

            // 2. Markdown Tables (| col1 | col2 |)
            if trimmed.starts_with('|') && trimmed.ends_with('|') {
                let mut table_rows = vec![line];
                while let Some(&next_line) = raw_lines.peek() {
                    let next_trimmed = next_line.trim();
                    if next_trimmed.starts_with('|') && next_trimmed.ends_with('|') {
                        table_rows.push(raw_lines.next().unwrap());
                    } else {
                        break;
                    }
                }

                Self::render_table(&table_rows, max_width, &mut lines);
                continue;
            }

            // 3. Headings
            if let Some(heading) = trimmed.strip_prefix("# ") {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        heading.to_string(),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                ]));
                let bar_len = max_width.min(90).saturating_sub(4);
                lines.push(Line::from(Span::styled(
                    format!("  {:━<bar_len$}", "", bar_len = bar_len),
                    Style::default().fg(Color::Rgb(56, 189, 248)),
                )));
                lines.push(Line::from(""));
                continue;
            }

            if let Some(heading) = trimmed.strip_prefix("## ") {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        heading.to_string(),
                        Style::default().fg(Color::Rgb(125, 211, 252)).add_modifier(Modifier::BOLD),
                    ),
                ]));
                let bar_len = max_width.min(75).saturating_sub(4);
                lines.push(Line::from(Span::styled(
                    format!("  {:─<bar_len$}", "", bar_len = bar_len),
                    Style::default().fg(Color::Rgb(60, 80, 110)),
                )));
                lines.push(Line::from(""));
                continue;
            }

            if let Some(heading) = trimmed.strip_prefix("### ") {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(
                        heading.to_string(),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                ]));
                continue;
            }

            // 4. GitHub-style Callouts / Alerts (> [!NOTE], > [!WARNING], > [!TIP], > [!IMPORTANT])
            if trimmed.starts_with("> [!NOTE]") {
                lines.push(Line::from(vec![
                    Span::styled("  ┃ ", Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD)),
                    Span::styled("ℹ NOTE", Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD)),
                ]));
                continue;
            }
            if trimmed.starts_with("> [!WARNING]") || trimmed.starts_with("> [!CAUTION]") {
                lines.push(Line::from(vec![
                    Span::styled("  ┃ ", Style::default().fg(Theme::STATUS_RISK_OPEN).add_modifier(Modifier::BOLD)),
                    Span::styled("▲ WARNING", Style::default().fg(Theme::STATUS_RISK_OPEN).add_modifier(Modifier::BOLD)),
                ]));
                continue;
            }
            if trimmed.starts_with("> [!TIP]") {
                lines.push(Line::from(vec![
                    Span::styled("  ┃ ", Style::default().fg(Theme::STATUS_ACCEPTED).add_modifier(Modifier::BOLD)),
                    Span::styled("★ TIP", Style::default().fg(Theme::STATUS_ACCEPTED).add_modifier(Modifier::BOLD)),
                ]));
                continue;
            }
            if trimmed.starts_with("> [!IMPORTANT]") {
                lines.push(Line::from(vec![
                    Span::styled("  ┃ ", Style::default().fg(Color::LightMagenta).add_modifier(Modifier::BOLD)),
                    Span::styled("◆ IMPORTANT", Style::default().fg(Color::LightMagenta).add_modifier(Modifier::BOLD)),
                ]));
                continue;
            }
            if let Some(quote) = trimmed.strip_prefix("> ") {
                let quote_spans = Self::format_inline_spans(quote);
                let wrapped = Self::wrap_spans(
                    quote_spans,
                    content_width,
                    vec![Span::styled("  ┃ ", Style::default().fg(Color::Rgb(90, 100, 120)))],
                    4,
                    vec![Span::styled("  ┃ ", Style::default().fg(Color::Rgb(90, 100, 120)))],
                    4,
                );
                lines.extend(wrapped);
                continue;
            }

            // 5. Task lists (- [ ], - [x])
            if let Some(task) = trimmed.strip_prefix("- [x] ") {
                let task_spans = Self::format_inline_spans(task);
                let wrapped = Self::wrap_spans(
                    task_spans,
                    content_width,
                    vec![
                        Span::raw("    "),
                        Span::styled("[✓] ", Style::default().fg(Theme::STATUS_ACCEPTED).add_modifier(Modifier::BOLD)),
                    ],
                    8,
                    vec![Span::raw("        ")],
                    8,
                );
                lines.extend(wrapped);
                continue;
            }
            if let Some(task) = trimmed.strip_prefix("- [ ] ") {
                let task_spans = Self::format_inline_spans(task);
                let wrapped = Self::wrap_spans(
                    task_spans,
                    content_width,
                    vec![
                        Span::raw("    "),
                        Span::styled("[ ] ", Style::default().fg(Theme::TEXT_MUTED)),
                    ],
                    8,
                    vec![Span::raw("        ")],
                    8,
                );
                lines.extend(wrapped);
                continue;
            }

            // 6. Numbered Lists (1. item, 2. item)
            if let Some((leading, num, item_text)) = Self::parse_numbered_item(line) {
                let num_str = format!("{}. ", num);
                let num_len = num_str.chars().count();
                let base_indent = 4 + (leading / 2) * 2;
                let first_indent_str = " ".repeat(base_indent);
                let hanging_indent_str = " ".repeat(base_indent + num_len);
                let total_hanging = base_indent + num_len;

                let item_spans = Self::format_inline_spans(item_text);
                let wrapped = Self::wrap_spans(
                    item_spans,
                    content_width,
                    vec![
                        Span::raw(first_indent_str),
                        Span::styled(num_str, Style::default().fg(Color::Rgb(253, 224, 71)).add_modifier(Modifier::BOLD)),
                    ],
                    total_hanging,
                    vec![Span::raw(hanging_indent_str)],
                    total_hanging,
                );

                let is_multiline = wrapped.len() > 1;
                lines.extend(wrapped);
                if is_multiline {
                    lines.push(Line::from(""));
                }
                continue;
            }

            // 7. Unordered list bullets (- item, * item, + item) with indentation levels
            if let Some((leading, bullet_text)) = Self::parse_bullet_item(line) {
                let level = leading / 2;
                let (bullet_symbol, bullet_style, base_spaces) = match level {
                    0 => ("• ", Style::default().fg(Color::Cyan), 4),
                    1 => ("◦ ", Style::default().fg(Color::Rgb(147, 197, 253)), 6),
                    _ => ("▪ ", Style::default().fg(Color::Rgb(156, 163, 175)), 8),
                };

                let first_indent_str = " ".repeat(base_spaces);
                let hanging_spaces = base_spaces + 2;
                let hanging_indent_str = " ".repeat(hanging_spaces);

                let bullet_spans = Self::format_inline_spans(bullet_text);
                let wrapped = Self::wrap_spans(
                    bullet_spans,
                    content_width,
                    vec![
                        Span::raw(first_indent_str),
                        Span::styled(bullet_symbol, bullet_style),
                    ],
                    hanging_spaces,
                    vec![Span::raw(hanging_indent_str)],
                    hanging_spaces,
                );

                let is_multiline = wrapped.len() > 1;
                lines.extend(wrapped);
                if is_multiline {
                    lines.push(Line::from(""));
                }
                continue;
            }

            // 8. Horizontal Rules (---, ***)
            if (trimmed == "---" || trimmed == "***" || trimmed == "___") && trimmed.len() >= 3 {
                let bar_len = content_width.saturating_sub(2);
                lines.push(Line::from(Span::styled(
                    format!("  {:─<bar_len$}", "", bar_len = bar_len),
                    Style::default().fg(Color::Rgb(50, 50, 65)),
                )));
                continue;
            }

            // 9. General body paragraphs
            if trimmed.is_empty() {
                lines.push(Line::from(""));
            } else {
                let body_spans = Self::format_inline_spans(trimmed);
                let wrapped = Self::wrap_spans(
                    body_spans,
                    content_width,
                    vec![Span::raw("  ")],
                    2,
                    vec![Span::raw("  ")],
                    2,
                );
                lines.extend(wrapped);
            }
        }

        lines
    }

    /// Renders markdown table rows as structured, highly readable records enclosed in clean borders.
    fn render_table(raw_rows: &[&str], max_width: usize, out_lines: &mut Vec<Line<'static>>) {
        if raw_rows.is_empty() {
            return;
        }

        // Parse cells for each row
        let parsed_rows: Vec<Vec<String>> = raw_rows
            .iter()
            .map(|row| {
                row.trim()
                    .trim_matches('|')
                    .split('|')
                    .map(|cell| cell.trim().to_string())
                    .collect()
            })
            .filter(|cells: &Vec<String>| {
                !cells.iter().all(|c| c.chars().all(|ch| ch == '-' || ch == ':'))
            })
            .collect();

        if parsed_rows.is_empty() {
            return;
        }

        let headers = &parsed_rows[0];
        let data_rows = &parsed_rows[1..];
        let card_width = max_width.max(30);
        let border_style = Style::default().fg(Color::Rgb(75, 85, 110));

        out_lines.push(Line::from(""));

        // Render each row as a distinct structured card
        for (row_idx, row) in data_rows.iter().enumerate() {
            let primary_title = if !row.is_empty() {
                Self::clean_inline_markers(&row[0])
            } else {
                format!("Record {}", row_idx + 1)
            };

            // Card Header: ┌─ [ Title ] ──────────────────┐
            let title_len = primary_title.chars().count();
            let header_overhead = 7 + title_len + 3 + 1; // "  ┌─ [ " (7) + " ] " (3) + "┐" (1)
            let remaining_dashes = card_width.saturating_sub(header_overhead);
            out_lines.push(Line::from(vec![
                Span::styled("  ┌─ [ ", border_style),
                Span::styled(primary_title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" ] {:─<dashes$}┐", "", dashes = remaining_dashes), border_style),
            ]));

            // Card Fields (skip first column which became title)
            // Available space inside borders:
            // Left: "  │ " (4 chars), Right: " │" (2 chars) -> inner width = card_width - 6
            let inner_width = card_width.saturating_sub(6);

            for (col_idx, cell) in row.iter().enumerate().skip(1) {
                if col_idx < headers.len() {
                    let field_name = Self::normalize_header_name(&headers[col_idx]);
                    let field_value = Self::clean_inline_markers(cell);

                    // Semantic badge formatting for status
                    let is_verified = field_value.to_lowercase().contains("verified");
                    let value_style = if is_verified {
                        Theme::badge_accepted()
                    } else if field_name.to_lowercase() == "status" {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else if field_name.to_lowercase() == "dependency" {
                        Style::default().fg(Color::LightCyan)
                    } else {
                        Style::default().fg(Color::Rgb(220, 225, 235))
                    };

                    let label_str = format!("{:<14}: ", field_name);
                    let label_len = label_str.chars().count();
                    let val_width = inner_width.saturating_sub(label_len);

                    // Word wrap the value text to val_width
                    let words: Vec<&str> = field_value.split_whitespace().collect();
                    let mut value_lines: Vec<String> = Vec::new();
                    let mut cur_line = String::new();

                    for w in words {
                        if cur_line.is_empty() {
                            cur_line.push_str(w);
                        } else if cur_line.chars().count() + 1 + w.chars().count() <= val_width {
                            cur_line.push(' ');
                            cur_line.push_str(w);
                        } else {
                            value_lines.push(cur_line);
                            cur_line = w.to_string();
                        }
                    }
                    if !cur_line.is_empty() {
                        value_lines.push(cur_line);
                    }
                    if value_lines.is_empty() {
                        value_lines.push(String::new());
                    }

                    for (vl_idx, v_line) in value_lines.into_iter().enumerate() {
                        let is_first = vl_idx == 0;
                        let prefix_label = if is_first {
                            label_str.clone()
                        } else {
                            " ".repeat(label_len)
                        };

                        let content_len = label_len + v_line.chars().count();
                        let pad_len = inner_width.saturating_sub(content_len);

                        out_lines.push(Line::from(vec![
                            Span::styled("  │ ", border_style),
                            Span::styled(
                                prefix_label,
                                if is_first {
                                    Style::default().fg(Theme::TEXT_MUTED).add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default()
                                },
                            ),
                            Span::styled(v_line, value_style),
                            Span::raw(" ".repeat(pad_len)),
                            Span::styled(" │", border_style),
                        ]));
                    }
                }
            }

            // Card Bottom: └────────────────────────────┘
            let bottom_dashes = card_width.saturating_sub(4);
            out_lines.push(Line::from(Span::styled(
                format!("  └{:─<bottom_dashes$}┘", "", bottom_dashes = bottom_dashes),
                border_style,
            )));
            out_lines.push(Line::from(""));
        }
    }

    /// Normalizes long column headers into clean, predictable labels.
    fn normalize_header_name(header: &str) -> String {
        let lower = header.to_lowercase();
        if lower.contains("demonstrable outcome") || lower.contains("exit gate") {
            "Outcome / Gate".to_string()
        } else if lower.contains("milestone") {
            "Milestone".to_string()
        } else if lower.contains("dependency") {
            "Dependency".to_string()
        } else if lower.contains("status") {
            "Status".to_string()
        } else if lower.contains("evidence") {
            "Evidence".to_string()
        } else if header.chars().count() > 14 {
            format!("{}…", &header[..13])
        } else {
            header.to_string()
        }
    }

    /// Strips markdown links like [Title](url) -> Title for clean card headers
    fn clean_inline_markers(text: &str) -> String {
        let mut cleaned = text.replace("**", "").replace('`', "");
        while let Some(start) = cleaned.find('[') {
            if let Some(mid) = cleaned[start..].find("](") {
                let mid_idx = start + mid;
                if let Some(end) = cleaned[mid_idx..].find(')') {
                    let end_idx = mid_idx + end;
                    let label = &cleaned[start + 1..mid_idx];
                    cleaned = format!("{}{}{}", &cleaned[..start], label, &cleaned[end_idx + 1..]);
                    continue;
                }
            }
            break;
        }
        cleaned
    }

    /// Parses numbered list line like "1. Review...", returns (leading_spaces, number, remaining_text)
    fn parse_numbered_item(line: &str) -> Option<(usize, usize, &str)> {
        let leading_spaces = line.chars().take_while(|c| *c == ' ').count();
        let trimmed = line.trim_start();
        let dot_pos = trimmed.find('.')?;
        if dot_pos == 0 || dot_pos > 4 {
            return None;
        }
        let num_str = &trimmed[..dot_pos];
        if !num_str.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let after_dot = &trimmed[dot_pos + 1..];
        if !after_dot.starts_with(' ') {
            return None;
        }
        let num: usize = num_str.parse().ok()?;
        let text = after_dot.trim_start();
        Some((leading_spaces, num, text))
    }

    /// Parses bullet list line like "- item" or "  * item", returns (leading_spaces, remaining_text)
    fn parse_bullet_item(line: &str) -> Option<(usize, &str)> {
        let leading_spaces = line.chars().take_while(|c| *c == ' ').count();
        let trimmed = line.trim_start();
        let text = if let Some(rest) = trimmed.strip_prefix("- ") {
            rest
        } else if let Some(rest) = trimmed.strip_prefix("* ") {
            rest
        } else if let Some(rest) = trimmed.strip_prefix("+ ") {
            rest
        } else {
            return None;
        };
        if text.trim().is_empty() {
            return None;
        }
        // Don't mistake hr "---" or "***" for bullet
        if trimmed.chars().all(|c| c == '-' || c == '*' || c == ' ') && trimmed.len() >= 3 {
            return None;
        }
        Some((leading_spaces, text))
    }

    /// Wraps styled spans into lines respecting maximum width, with distinct first-line and hanging prefixes.
    fn wrap_spans(
        spans: Vec<Span<'static>>,
        max_width: usize,
        first_prefix: Vec<Span<'static>>,
        first_prefix_len: usize,
        hanging_prefix: Vec<Span<'static>>,
        hanging_prefix_len: usize,
    ) -> Vec<Line<'static>> {
        // Break spans into styled words
        let mut words: Vec<StyledWord> = Vec::new();
        for span in spans {
            for word in span.content.split_whitespace() {
                words.push(StyledWord {
                    text: word.to_string(),
                    style: span.style,
                });
            }
        }

        if words.is_empty() {
            return vec![Line::from(first_prefix)];
        }

        let mut lines = Vec::new();
        let mut cur_line_spans = first_prefix.clone();
        let mut cur_line_len = first_prefix_len;
        let mut is_first_line = true;
        let mut prev_style: Option<Style> = None;

        for word in words {
            let word_len = word.text.chars().count();
            let cur_prefix_len = if is_first_line { first_prefix_len } else { hanging_prefix_len };
            let has_content = cur_line_len > cur_prefix_len;
            let space_needed = if has_content { 1 } else { 0 };

            if has_content && cur_line_len + space_needed + word_len > max_width {
                lines.push(Line::from(cur_line_spans));
                cur_line_spans = hanging_prefix.clone();
                cur_line_len = hanging_prefix_len;
                is_first_line = false;
                prev_style = None;
            }

            if cur_line_len > (if is_first_line { first_prefix_len } else { hanging_prefix_len }) {
                let space_style = if let Some(prev) = prev_style {
                    if prev == word.style { prev } else { Style::default() }
                } else {
                    Style::default()
                };
                cur_line_spans.push(Span::styled(" ", space_style));
                cur_line_len += 1;
            }

            cur_line_spans.push(Span::styled(word.text, word.style));
            cur_line_len += word_len;
            prev_style = Some(word.style);
        }

        if cur_line_len > (if is_first_line { first_prefix_len } else { hanging_prefix_len }) {
            lines.push(Line::from(cur_line_spans));
        }

        lines
    }

    /// Parses inline formatting: [link](url), **bold**, `code`, and *italic*
    pub fn format_inline_spans(text: &str) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        let mut current = String::new();
        let mut in_code = false;
        let mut in_bold = false;
        let mut in_italic = false;

        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // 1. Markdown Links: [label](url)
            if !in_code && chars[i] == '[' {
                if let Some(close_bracket) = chars[i..].iter().position(|&c| c == ']') {
                    let after_bracket = i + close_bracket + 1;
                    if after_bracket < chars.len() && chars[after_bracket] == '(' {
                        if let Some(close_paren) = chars[after_bracket..].iter().position(|&c| c == ')') {
                            if !current.is_empty() {
                                spans.push(Self::create_span(&current, in_code, in_bold, in_italic));
                                current.clear();
                            }

                            let label: String = chars[i + 1..i + close_bracket].iter().collect();
                            spans.push(Span::styled(
                                format!("{} ↗", label),
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::UNDERLINED),
                            ));

                            i = after_bracket + close_paren + 1;
                            continue;
                        }
                    }
                }
            }

            // 2. Bold: **text** or __text__
            if !in_code && i + 1 < chars.len() && ((chars[i] == '*' && chars[i + 1] == '*') || (chars[i] == '_' && chars[i + 1] == '_')) {
                if !current.is_empty() {
                    spans.push(Self::create_span(&current, in_code, in_bold, in_italic));
                    current.clear();
                }
                in_bold = !in_bold;
                i += 2;
                continue;
            }

            // 3. Inline Code: `text`
            if chars[i] == '`' {
                if !current.is_empty() {
                    spans.push(Self::create_span(&current, in_code, in_bold, in_italic));
                    current.clear();
                }
                in_code = !in_code;
                i += 1;
                continue;
            }

            // 4. Italic: *text* (single asterisk)
            if !in_code && chars[i] == '*' {
                if !current.is_empty() {
                    spans.push(Self::create_span(&current, in_code, in_bold, in_italic));
                    current.clear();
                }
                in_italic = !in_italic;
                i += 1;
                continue;
            }

            current.push(chars[i]);
            i += 1;
        }

        if !current.is_empty() {
            spans.push(Self::create_span(&current, in_code, in_bold, in_italic));
        }

        spans
    }

    fn create_span(text: &str, in_code: bool, in_bold: bool, in_italic: bool) -> Span<'static> {
        let mut style = Style::default();

        if in_code {
            style = style
                .bg(Color::Rgb(30, 41, 59)) // Slate background
                .fg(Color::Rgb(253, 224, 71)) // Light Yellow
                .add_modifier(Modifier::BOLD);
        } else if in_bold {
            style = style
                .fg(Color::Rgb(254, 240, 138)) // Bright Warm Yellow for bold text to pop
                .add_modifier(Modifier::BOLD);
        } else if in_italic {
            style = style
                .fg(Color::Rgb(203, 213, 225)) // Light Slate
                .add_modifier(Modifier::ITALIC);
        } else {
            style = style.fg(Color::Rgb(226, 232, 240)); // Clean Slate White
        }

        Span::styled(text.to_string(), style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_markdown_strips_metadata() {
        let raw = r#"---hyperkb
{
  "id": "dec_2026_001",
  "kind": "decision",
  "status": "accepted",
  "owner": "wiqar"
}
---
# Architecture

This is **important** and uses `rustc`.
"#;
        let formatted = MarkdownFormatter::format_markdown(raw, 80);
        let any_has_hyperkb = formatted.iter().any(|l| {
            l.spans.iter().any(|s| s.content.contains("---hyperkb"))
        });
        assert!(!any_has_hyperkb);

        let has_heading = formatted.iter().any(|l| {
            l.spans.iter().any(|s| s.content.contains("Architecture"))
        });
        assert!(has_heading);
    }

    #[test]
    fn test_format_markdown_links() {
        let raw = "See the [Product Spec](specs/v0.md) for details.";
        let spans = MarkdownFormatter::format_inline_spans(raw);
        let has_link = spans.iter().any(|s| s.content.contains("Product Spec ↗"));
        assert!(has_link);
        let has_raw_url = spans.iter().any(|s| s.content.contains("specs/v0.md"));
        assert!(!has_raw_url);
    }

    #[test]
    fn test_format_markdown_table_cards() {
        let raw_table = vec![
            "| Milestone | Status |",
            "|---|---|",
            "| M0 — Core | Verified |",
        ];
        let mut out = Vec::new();
        MarkdownFormatter::render_table(&raw_table, 80, &mut out);
        assert!(!out.is_empty());
        let has_card_title = out.iter().any(|l| l.spans.iter().any(|s| s.content.contains("M0 — Core")));
        assert!(has_card_title);
    }

    #[test]
    fn test_format_markdown_numbered_list() {
        let raw = "1. First step in the plan\n2. Second step in the plan";
        let lines = MarkdownFormatter::format_markdown(raw, 80);
        let has_one = lines.iter().any(|l| l.spans.iter().any(|s| s.content.contains("1. ")));
        let has_two = lines.iter().any(|l| l.spans.iter().any(|s| s.content.contains("2. ")));
        assert!(has_one);
        assert!(has_two);
    }

    #[test]
    fn test_format_markdown_bullet_hanging_indent() {
        let raw = "- A very long bullet point that should wrap across lines and maintain indentation cleanly.";
        let lines = MarkdownFormatter::format_markdown(raw, 40);
        assert!(lines.len() >= 2);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("• ")));
        assert!(lines[1].spans[0].content.starts_with("      "));
    }

    #[test]
    fn test_format_real_roadmap_table_has_enclosed_borders() {
        let raw = r#"| Milestone | Dependency | Demonstrable outcome / exit gate | Status | Evidence |
|---|---|---|---|---|
| M0 — Approve architecture and source inventory | None | Owner authorized Go work and accepted repo-document/private-memory authority, portable identity split and typed SQL boundary. Read-only source/config/writer/schema inventory and synthetic fixture design exist; two existing SQLite schemas were later inspected separately with owner permission. | Verified for bounded implementation | Spec, architecture, source inventory, evidence |"#;
        let lines = MarkdownFormatter::format_markdown(raw, 80);
        let card_lines: Vec<&Line> = lines.iter().filter(|l| !l.spans.is_empty() && !l.spans[0].content.trim().is_empty()).collect();
        for line in &card_lines {
            let first_span = &line.spans[0].content;
            let last_span = &line.spans.last().unwrap().content;
            assert!(first_span.starts_with("  ┌") || first_span.starts_with("  │") || first_span.starts_with("  └"),
                "Line escaped left card border: {:?}", line);
            assert!(last_span.ends_with('┐') || last_span.ends_with('│') || last_span.ends_with('┘'),
                "Line escaped right card border: {:?}", line);
        }
    }

    #[test]
    fn test_format_dense_bullets_have_hanging_indent_and_spacing() {
        let raw = r#"## Observed passing commands

- `go fmt ./... && make check && make build` passed on macOS/amd64 after the final local changes. `make check` runs tagged Go tests, race tests and vet. Targeted compiled PTY setup/search, linked decision navigation and reviewed repair rehearsals passed on macOS; the real OpenCode PTY repair was separately observed earlier. No pre-existing KB was used.
- On Linux/amd64, the local Docker engine ran `golang:1.27.1-bookworm` (image digest `sha256:69a7b9788769bec032d238959b61854e9ae87f57be9029ec04e9885fabf99195`) with **network disabled**, source mounted read-only and the pinned module cache mounted read-only."#;
        let lines = MarkdownFormatter::format_markdown(raw, 70);
        let has_hanging = lines.iter().any(|l| l.spans.first().map(|s| s.content.starts_with("      ")).unwrap_or(false));
        assert!(has_hanging, "Expected hanging indent on wrapped bullet continuation lines");
        let has_empty = lines.iter().any(|l| l.spans.is_empty() || l.spans.iter().all(|s| s.content.trim().is_empty()));
        assert!(has_empty, "Expected blank line between multi-line bullets");
    }
}
