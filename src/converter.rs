use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, CodeBlockKind};
use crate::block::Block;

#[derive(Debug, Clone)]
pub struct ConvertOptions {
    pub flat_headings: bool,
    pub heading_markers: bool,
    pub split_paragraphs: bool,
    /// When true: nest code blocks under the immediately preceding paragraph block.
    pub code_under_para: bool,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        ConvertOptions {
            flat_headings: false,
            heading_markers: true,
            split_paragraphs: false,
            code_under_para: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Heading context — stays open until a same-or-higher heading closes it.
// ---------------------------------------------------------------------------
struct HeadingFrame {
    level: u32,
    text: String,
    children: Vec<Block>,
}

// ---------------------------------------------------------------------------
// Inline/block container frames (lists, blockquotes, code, tables).
// ---------------------------------------------------------------------------
enum Frame {
    Container { children: Vec<Block> },
    ListItem  { text: String, children: Vec<Block> },
    Code      { lang: String, lines: String },
    Image     { url: String, alt: String },
    /// Table accumulates rows as raw Markdown strings, rendered as a single multiline block.
    Table     { col_count: usize, rows: Vec<String>, current_row: Vec<String>, current_cell: String },
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------
pub fn convert(input: &str, opts: &ConvertOptions) -> Vec<Block> {
    let mut md_opts = Options::empty();
    md_opts.insert(Options::ENABLE_TABLES);
    md_opts.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(input, md_opts);

    let mut roots: Vec<Block> = Vec::new();
    let mut h_stack: Vec<HeadingFrame> = Vec::new();
    let mut f_stack: Vec<Frame> = Vec::new();
    let mut inline_buf = String::new();
    let mut in_heading = false;
    let mut link_url_stack: Vec<String> = Vec::new();

    for event in parser {
        match event {
            // ---------------------------------------------------------------
            // Headings
            // ---------------------------------------------------------------
            Event::Start(Tag::Heading { level, .. }) => {
                let level = level as u32;
                flush_headings(&mut h_stack, &mut roots, level);
                h_stack.push(HeadingFrame { level, text: String::new(), children: Vec::new() });
                in_heading = true;
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
            }

            // ---------------------------------------------------------------
            // Paragraphs
            // ---------------------------------------------------------------
            Event::Start(Tag::Paragraph) => {
                inline_buf.clear();
            }
            Event::End(TagEnd::Paragraph) => {
                let text = std::mem::take(&mut inline_buf);
                if text.trim().is_empty() { continue; }
                if opts.split_paragraphs {
                    for line in text.split('\n') {
                        let t = line.trim();
                        if !t.is_empty() {
                            append_block(&mut f_stack, &mut h_stack, &mut roots, Block::new(t));
                        }
                    }
                } else {
                    append_block(&mut f_stack, &mut h_stack, &mut roots, Block::new(text.trim()));
                }
            }

            // ---------------------------------------------------------------
            // Block quotes
            // ---------------------------------------------------------------
            Event::Start(Tag::BlockQuote(_)) => {
                f_stack.push(Frame::Container { children: Vec::new() });
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                if let Some(Frame::Container { children }) = f_stack.pop() {
                    for b in children {
                        append_block(&mut f_stack, &mut h_stack, &mut roots, b);
                    }
                }
            }

            // ---------------------------------------------------------------
            // Lists — items nest under the immediately preceding paragraph.
            // ---------------------------------------------------------------
            Event::Start(Tag::List(_)) => {
                f_stack.push(Frame::Container { children: Vec::new() });
            }
            Event::End(TagEnd::List(_)) => {
                if let Some(Frame::Container { children: list_children }) = f_stack.pop() {
                    // Attach to the preceding non-list-item block (e.g. a paragraph).
                    // If the preceding block is itself a list item, don't steal it
                    // (two consecutive lists should remain siblings, not nest).
                    let preceding = pop_last_paragraph(&mut f_stack, &mut h_stack, &mut roots);
                    match preceding {
                        Some(mut parent) => {
                            for c in list_children { parent.add_child(c); }
                            append_block(&mut f_stack, &mut h_stack, &mut roots, parent);
                        }
                        None => {
                            for c in list_children {
                                append_block(&mut f_stack, &mut h_stack, &mut roots, c);
                            }
                        }
                    }
                }
            }
            Event::Start(Tag::Item) => {
                f_stack.push(Frame::ListItem { text: String::new(), children: Vec::new() });
            }
            Event::End(TagEnd::Item) => {
                if let Some(Frame::ListItem { text, children }) = f_stack.pop() {
                    let mut b = Block::new_list_item(text.trim());
                    for c in children { b.add_child(c); }
                    append_block(&mut f_stack, &mut h_stack, &mut roots, b);
                }
            }

            // ---------------------------------------------------------------
            // Code blocks — rendered as a single multiline block, not child blocks.
            // Logseq format:  - ```lang
            //                   line1
            //                   line2
            //                   ```
            // ---------------------------------------------------------------
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(s) => s.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                f_stack.push(Frame::Code { lang, lines: String::new() });
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(Frame::Code { lang, lines }) = f_stack.pop() {
                    let mut content = format!("```{}", lang);
                    let code = lines.trim_end_matches('\n');
                    if !code.is_empty() {
                        content.push('\n');
                        content.push_str(code);
                    }
                    content.push_str("\n```");
                    let code_block = Block::new(content);
                    if opts.code_under_para {
                        let preceding = pop_last_paragraph(&mut f_stack, &mut h_stack, &mut roots);
                        match preceding {
                            Some(mut parent) => {
                                parent.add_child(code_block);
                                append_block(&mut f_stack, &mut h_stack, &mut roots, parent);
                            }
                            None => append_block(&mut f_stack, &mut h_stack, &mut roots, code_block),
                        }
                    } else {
                        append_block(&mut f_stack, &mut h_stack, &mut roots, code_block);
                    }
                }
            }

            // ---------------------------------------------------------------
            // Images — preserve as ![alt](url) inline or standalone block.
            // ---------------------------------------------------------------
            Event::Start(Tag::Image { dest_url, .. }) => {
                f_stack.push(Frame::Image { url: dest_url.to_string(), alt: String::new() });
            }
            Event::End(TagEnd::Image) => {
                if let Some(Frame::Image { url, alt }) = f_stack.pop() {
                    let img = format!("![{}]({})", alt, url);
                    // Images may appear inside a paragraph (inline_buf) or standalone.
                    route_text(&mut f_stack, &mut h_stack, &mut inline_buf, in_heading, &img);
                }
            }

            // ---------------------------------------------------------------
            // Tables — rendered as a single multiline block preserving Markdown syntax.
            // ---------------------------------------------------------------
            Event::Start(Tag::Table(alignments)) => {
                f_stack.push(Frame::Table {
                    col_count: alignments.len(),
                    rows: Vec::new(),
                    current_row: Vec::new(),
                    current_cell: String::new(),
                });
            }
            Event::End(TagEnd::Table) => {
                if let Some(Frame::Table { col_count, rows, .. }) = f_stack.pop() {
                    if rows.is_empty() { continue; }
                    // rows[0] = header, rows[1] = separator, rows[2..] = data
                    // Build the full Markdown table as a single multiline block.
                    let mut lines: Vec<String> = Vec::new();
                    for (i, row) in rows.iter().enumerate() {
                        lines.push(row.clone());
                        // Insert separator after header row.
                        if i == 0 {
                            let sep = (0..col_count).map(|_| "---").collect::<Vec<_>>().join(" | ");
                            lines.push(format!("| {} |", sep));
                        }
                    }
                    let content = lines.join("\n");
                    append_block(&mut f_stack, &mut h_stack, &mut roots, Block::new(content));
                }
            }
            Event::Start(Tag::TableHead) | Event::Start(Tag::TableRow) => {}
            Event::End(TagEnd::TableHead) | Event::End(TagEnd::TableRow) => {
                if let Some(Frame::Table { rows, current_row, .. }) = f_stack.last_mut() {
                    let cells = std::mem::take(current_row);
                    rows.push(format!("| {} |", cells.join(" | ")));
                }
            }
            Event::Start(Tag::TableCell) => {
                if let Some(Frame::Table { current_cell, .. }) = f_stack.last_mut() {
                    current_cell.clear();
                }
            }
            Event::End(TagEnd::TableCell) => {
                if let Some(Frame::Table { current_row, current_cell, .. }) = f_stack.last_mut() {
                    let cell = std::mem::take(current_cell);
                    current_row.push(cell);
                }
            }

            // ---------------------------------------------------------------
            // Inline content
            // ---------------------------------------------------------------
            Event::Text(text) => route_text(&mut f_stack, &mut h_stack, &mut inline_buf, in_heading, &text),
            Event::Code(code) => {
                let snippet = format!("`{}`", code);
                route_text(&mut f_stack, &mut h_stack, &mut inline_buf, in_heading, &snippet);
            }
            Event::Start(Tag::Strong)         => route_text(&mut f_stack, &mut h_stack, &mut inline_buf, in_heading, "**"),
            Event::End(TagEnd::Strong)         => route_text(&mut f_stack, &mut h_stack, &mut inline_buf, in_heading, "**"),
            Event::Start(Tag::Emphasis)        => route_text(&mut f_stack, &mut h_stack, &mut inline_buf, in_heading, "*"),
            Event::End(TagEnd::Emphasis)       => route_text(&mut f_stack, &mut h_stack, &mut inline_buf, in_heading, "*"),
            Event::Start(Tag::Strikethrough)   => route_text(&mut f_stack, &mut h_stack, &mut inline_buf, in_heading, "~~"),
            Event::End(TagEnd::Strikethrough)  => route_text(&mut f_stack, &mut h_stack, &mut inline_buf, in_heading, "~~"),
            Event::Start(Tag::Link { dest_url, .. }) => {
                link_url_stack.push(dest_url.to_string());
                route_text(&mut f_stack, &mut h_stack, &mut inline_buf, in_heading, "[");
            }
            Event::End(TagEnd::Link) => {
                let url = link_url_stack.pop().unwrap_or_default();
                route_text(&mut f_stack, &mut h_stack, &mut inline_buf, in_heading, &format!("]({})", url));
            }
            Event::SoftBreak => {
                let sep = if opts.split_paragraphs { "\n" } else { " " };
                route_text(&mut f_stack, &mut h_stack, &mut inline_buf, in_heading, sep);
            }
            Event::HardBreak => route_text(&mut f_stack, &mut h_stack, &mut inline_buf, in_heading, "\n"),
            Event::Html(html) | Event::InlineHtml(html) => {
                route_text(&mut f_stack, &mut h_stack, &mut inline_buf, in_heading, &html);
            }
            Event::Rule => {} // skip
            _ => {}
        }
    }

    flush_headings(&mut h_stack, &mut roots, 0);

    if opts.flat_headings {
        for b in &mut roots { make_flat(b); }
    }
    if opts.heading_markers {
        for b in &mut roots { apply_heading_markers(b); }
    }

    roots
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn flush_headings(h_stack: &mut Vec<HeadingFrame>, roots: &mut Vec<Block>, up_to_level: u32) {
    while let Some(top) = h_stack.last() {
        if top.level >= up_to_level || up_to_level == 0 {
            let frame = h_stack.pop().unwrap();
            let mut b = Block::new_heading(&frame.text, frame.level);
            for c in frame.children { b.add_child(c); }
            match h_stack.last_mut() {
                Some(parent) => parent.children.push(b),
                None => roots.push(b),
            }
        } else {
            break;
        }
    }
}

fn append_block(
    f_stack: &mut Vec<Frame>,
    h_stack: &mut Vec<HeadingFrame>,
    roots: &mut Vec<Block>,
    block: Block,
) {
    for frame in f_stack.iter_mut().rev() {
        match frame {
            Frame::Container { children } | Frame::ListItem { children, .. } => {
                children.push(block);
                return;
            }
            _ => {}
        }
    }
    if let Some(h) = h_stack.last_mut() {
        h.children.push(block);
        return;
    }
    roots.push(block);
}

fn route_text(
    f_stack: &mut Vec<Frame>,
    h_stack: &mut Vec<HeadingFrame>,
    inline_buf: &mut String,
    in_heading: bool,
    s: &str,
) {
    for frame in f_stack.iter_mut().rev() {
        match frame {
            Frame::ListItem { text, .. }       => { text.push_str(s); return; }
            Frame::Code { lines, .. }          => { lines.push_str(s); return; }
            Frame::Image { alt, .. }           => { alt.push_str(s); return; }
            Frame::Table { current_cell, .. }  => { current_cell.push_str(s); return; }
            _ => {}
        }
    }
    if in_heading {
        if let Some(h) = h_stack.last_mut() {
            h.text.push_str(s);
            return;
        }
    }
    inline_buf.push_str(s);
}

fn make_flat(b: &mut Block) {
    if b.from_heading {
        b.content = format!("**{}**", b.content);
    }
    for child in &mut b.children {
        make_flat(child);
    }
}

/// Prefix heading blocks with `#` markers so Logseq renders them as styled headings.
/// e.g. level-2 heading "Section" → "## Section"
fn apply_heading_markers(b: &mut Block) {
    if b.from_heading && b.heading_level > 0 {
        let prefix = "#".repeat(b.heading_level as usize);
        b.content = format!("{} {}", prefix, b.content);
    }
    for child in &mut b.children {
        apply_heading_markers(child);
    }
}

/// Pop the last block from the current context IF it is a paragraph-like block
/// (i.e. not a list item). Used so that a list can nest under its preceding paragraph.
/// Returns None if the last block is a list item or if the context is empty.
fn pop_last_paragraph(
    f_stack: &mut Vec<Frame>,
    h_stack: &mut Vec<HeadingFrame>,
    roots: &mut Vec<Block>,
) -> Option<Block> {
    // Check innermost container first.
    for frame in f_stack.iter_mut().rev() {
        match frame {
            Frame::Container { children } | Frame::ListItem { children, .. } => {
                // Only steal if the last child is a paragraph (not a list item).
                if let Some(last) = children.last() {
                    if !last.from_list_item {
                        return children.pop();
                    }
                }
                return None;
            }
            _ => {}
        }
    }
    // Fall back to heading children or roots.
    if let Some(h) = h_stack.last_mut() {
        if let Some(last) = h.children.last() {
            if !last.from_list_item {
                return h.children.pop();
            }
        }
        return None;
    }
    if let Some(last) = roots.last() {
        if !last.from_list_item {
            return roots.pop();
        }
    }
    None
}

