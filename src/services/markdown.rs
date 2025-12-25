use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Clone, Debug)]
pub enum BlockKind {
    Paragraph,
    Heading(u32),
    CodeBlock,
    ListItem,
}

#[derive(Clone, Debug)]
pub struct RenderBlock {
    pub kind: BlockKind,
    pub text: String,
    pub bold: bool,
    pub italic: bool,
}

/// Parse markdown into simple block structures for rendering.
pub fn render_blocks(source: &str) -> Vec<RenderBlock> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(source, options);

    let mut blocks: Vec<RenderBlock> = Vec::new();
    let mut current = RenderBlock {
        kind: BlockKind::Paragraph,
        text: String::new(),
        bold: false,
        italic: false,
    };
    let mut bold_stack: u32 = 0;
    let mut italic_stack: u32 = 0;

    for event in parser {
        match event {
            Event::Start(Tag::Paragraph { .. }) => {
                current.text.clear();
                current.bold = false;
                current.italic = false;
                current.kind = BlockKind::Paragraph;
            }
            Event::Start(Tag::Item) => {
                current.text.clear();
                current.bold = false;
                current.italic = false;
                current.kind = BlockKind::ListItem;
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let heading_level = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                current.text.clear();
                current.kind = BlockKind::Heading(heading_level);
                current.bold = true;
            }
            Event::Text(t) => {
                current.text.push_str(&t);
            }
            Event::Code(t) => {
                current.text.push_str(&t);
            }
            Event::Start(Tag::CodeBlock(_)) => {
                current.text.clear();
                current.kind = BlockKind::CodeBlock;
            }
            Event::End(TagEnd::CodeBlock) => {
                if !current.text.trim().is_empty() {
                    blocks.push(current.clone());
                }
                current.text.clear();
                current.kind = BlockKind::Paragraph;
            }
            Event::End(TagEnd::Paragraph) | Event::End(TagEnd::Item) => {
                if !current.text.trim().is_empty() {
                    current.bold = current.bold || bold_stack > 0;
                    current.italic = current.italic || italic_stack > 0;
                    blocks.push(current.clone());
                }
                current.text.clear();
                current.bold = false;
                current.italic = false;
                bold_stack = 0;
                italic_stack = 0;
                current.kind = BlockKind::Paragraph;
            }
            Event::End(TagEnd::Heading { .. }) => {
                if !current.text.trim().is_empty() {
                    blocks.push(current.clone());
                }
                current.text.clear();
                current.bold = false;
                current.italic = false;
                bold_stack = 0;
                italic_stack = 0;
                current.kind = BlockKind::Paragraph;
            }
            Event::Start(Tag::Emphasis) => {
                italic_stack += 1;
                current.italic = true;
            }
            Event::End(TagEnd::Emphasis) => {
                italic_stack = italic_stack.saturating_sub(1);
            }
            Event::Start(Tag::Strong) => {
                bold_stack += 1;
                current.bold = true;
            }
            Event::End(TagEnd::Strong) => {
                bold_stack = bold_stack.saturating_sub(1);
            }
            Event::HardBreak | Event::SoftBreak => {
                current.text.push('\n');
            }
            _ => {}
        }
    }

    if !current.text.trim().is_empty() {
        blocks.push(current);
    }

    blocks
}
