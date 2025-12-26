use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Clone, Debug)]
pub enum Block {
    Paragraph(Vec<InlineRun>),
    Heading(u32, Vec<InlineRun>),
    ListItem(Vec<InlineRun>),
    OrderedListItem { number: u64, content: Vec<InlineRun> },
    TaskListItem { checked: bool, content: Vec<InlineRun> },
    CodeBlock(String),
    Quote(Vec<InlineRun>),
    Image { alt: String, src: String },
}

#[derive(Clone, Debug)]
pub struct InlineRun {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub link: Option<String>,
}

impl InlineRun {
    fn new(text: String, bold: bool, italic: bool, code: bool, link: Option<String>) -> Self {
        Self {
            text,
            bold,
            italic,
            code,
            link,
        }
    }
}

pub fn render_blocks(source: &str) -> Vec<Block> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(source, options);

    let mut blocks = Vec::new();
    let mut runs: Vec<InlineRun> = Vec::new();
    let mut bold_stack: u32 = 0;
    let mut italic_stack: u32 = 0;
    let mut link_stack: Vec<String> = Vec::new();
    let mut in_quote = false;
    let mut heading_level: Option<u32> = None;
    let mut in_list_item = false;
    let mut code_block: Option<String> = None;
    // Image parsing state: (src, alt_text_accumulator)
    let mut image_context: Option<(String, String)> = None;
    // Task list state: Some(checked) if inside a task list item
    let mut task_list_checked: Option<bool> = None;
    // Ordered list state: Some(counter) if inside an ordered list, increments per item
    let mut ordered_list_counter: Option<u64> = None;

    let push_runs_as = |target: &mut Vec<Block>, runs: &mut Vec<InlineRun>, kind: BlockKind| {
        if runs.is_empty() {
            return;
        }
        let block = match kind {
            BlockKind::Paragraph => Block::Paragraph(runs.clone()),
            BlockKind::Heading(level) => Block::Heading(level, runs.clone()),
            BlockKind::ListItem => Block::ListItem(runs.clone()),
            BlockKind::Quote => Block::Quote(runs.clone()),
        };
        target.push(block);
        runs.clear();
    };

    for event in parser {
        match event {
            Event::Start(Tag::Paragraph { .. }) => {
                runs.clear();
                heading_level = None;
                in_list_item = false;
            }
            Event::End(TagEnd::Paragraph) => {
                let kind = if in_quote {
                    BlockKind::Quote
                } else {
                    BlockKind::Paragraph
                };
                push_runs_as(&mut blocks, &mut runs, kind);
                bold_stack = 0;
                italic_stack = 0;
            }
            Event::Start(Tag::Heading { level, .. }) => {
                runs.clear();
                heading_level = Some(match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                });
            }
            Event::End(TagEnd::Heading { .. }) => {
                let lvl = heading_level.unwrap_or(1);
                push_runs_as(&mut blocks, &mut runs, BlockKind::Heading(lvl));
                heading_level = None;
                bold_stack = 0;
                italic_stack = 0;
            }
            Event::Start(Tag::List(start_number)) => {
                // start_number is Some(n) for ordered lists, None for unordered
                ordered_list_counter = start_number;
            }
            Event::End(TagEnd::List(_)) => {
                ordered_list_counter = None;
            }
            Event::Start(Tag::Item) => {
                runs.clear();
                in_list_item = true;
                task_list_checked = None;
            }
            Event::End(TagEnd::Item) => {
                if let Some(checked) = task_list_checked.take() {
                    // This is a task list item
                    if !runs.is_empty() {
                        blocks.push(Block::TaskListItem {
                            checked,
                            content: runs.clone(),
                        });
                        runs.clear();
                    }
                } else if let Some(ref mut counter) = ordered_list_counter {
                    // Ordered list item
                    if !runs.is_empty() {
                        blocks.push(Block::OrderedListItem {
                            number: *counter,
                            content: runs.clone(),
                        });
                        runs.clear();
                    }
                    *counter += 1;
                } else {
                    // Unordered list item
                    push_runs_as(&mut blocks, &mut runs, BlockKind::ListItem);
                }
                in_list_item = false;
            }
            Event::TaskListMarker(checked) => {
                task_list_checked = Some(checked);
            }
            Event::Start(Tag::BlockQuote(_)) => {
                in_quote = true;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                in_quote = false;
            }
            Event::Start(Tag::CodeBlock(_)) => {
                code_block = Some(String::new());
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(text) = code_block.take() {
                    blocks.push(Block::CodeBlock(text));
                }
            }
            Event::Text(t) => {
                if let Some(code) = code_block.as_mut() {
                    code.push_str(&t);
                    continue;
                }
                // Accumulate alt text if inside an image
                if let Some((_, ref mut alt)) = image_context {
                    alt.push_str(&t);
                    continue;
                }
                runs.push(InlineRun::new(
                    t.to_string(),
                    bold_stack > 0,
                    italic_stack > 0,
                    false,
                    link_stack.last().cloned(),
                ));
            }
            Event::Code(t) => {
                runs.push(InlineRun::new(
                    t.to_string(),
                    bold_stack > 0,
                    italic_stack > 0,
                    true,
                    link_stack.last().cloned(),
                ));
            }
            Event::Start(Tag::Emphasis) => {
                italic_stack += 1;
            }
            Event::End(TagEnd::Emphasis) => {
                italic_stack = italic_stack.saturating_sub(1);
            }
            Event::Start(Tag::Strong) => {
                bold_stack += 1;
            }
            Event::End(TagEnd::Strong) => {
                bold_stack = bold_stack.saturating_sub(1);
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                link_stack.push(dest_url.to_string());
            }
            Event::End(TagEnd::Link) => {
                link_stack.pop();
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                // Start collecting image: store src and prepare alt accumulator
                image_context = Some((dest_url.to_string(), String::new()));
            }
            Event::End(TagEnd::Image) => {
                // Finish image block with collected alt text
                if let Some((src, alt)) = image_context.take() {
                    blocks.push(Block::Image { alt, src });
                }
            }
            Event::HardBreak | Event::SoftBreak => {
                runs.push(InlineRun::new(
                    "\n".to_string(),
                    bold_stack > 0,
                    italic_stack > 0,
                    false,
                    link_stack.last().cloned(),
                ));
            }
            _ => {}
        }
    }

    if !runs.is_empty() {
        let kind = if in_quote {
            BlockKind::Quote
        } else if in_list_item {
            BlockKind::ListItem
        } else {
            BlockKind::Paragraph
        };
        push_runs_as(&mut blocks, &mut runs, kind);
    }

    blocks
}

pub enum BlockKind {
    Paragraph,
    Heading(u32),
    ListItem,
    Quote,
}
