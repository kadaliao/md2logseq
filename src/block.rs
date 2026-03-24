/// A single Logseq block node.
#[derive(Debug, Default)]
pub struct Block {
    pub content: String,
    pub children: Vec<Block>,
    pub from_heading: bool,
    pub heading_level: u32,  // 1-6 when from_heading, 0 otherwise
    pub from_list_item: bool,
}

impl Block {
    pub fn new(content: impl Into<String>) -> Self {
        Block { content: content.into(), children: Vec::new(), from_heading: false, heading_level: 0, from_list_item: false }
    }

    pub fn new_heading(content: impl Into<String>, level: u32) -> Self {
        Block { content: content.into(), children: Vec::new(), from_heading: true, heading_level: level, from_list_item: false }
    }

    pub fn new_list_item(content: impl Into<String>) -> Self {
        Block { content: content.into(), children: Vec::new(), from_heading: false, heading_level: 0, from_list_item: true }
    }

    pub fn add_child(&mut self, child: Block) {
        self.children.push(child);
    }

    /// Render this block and all children at the given indent level.
    /// Multi-line content (e.g. code blocks) is rendered with continuation lines
    /// indented under the bullet, not as child blocks.
    pub fn render(&self, indent: usize) -> String {
        let prefix = "  ".repeat(indent);
        let cont = format!("{}  ", prefix); // continuation indent (no "- ")
        let mut out = String::new();
        for (i, line) in self.content.split('\n').enumerate() {
            if i == 0 {
                out.push_str(&format!("{}- {}\n", prefix, line));
            } else {
                out.push_str(&format!("{}{}\n", cont, line));
            }
        }
        for child in &self.children {
            out.push_str(&child.render(indent + 1));
        }
        out
    }
}

/// Render a list of top-level blocks.
pub fn render_blocks(blocks: &[Block]) -> String {
    blocks.iter().map(|b| b.render(0)).collect()
}
