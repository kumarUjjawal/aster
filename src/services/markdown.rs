use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::collections::HashMap;

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
    /// Inline footnote reference marker [^label]
    FootnoteRef { label: String, index: usize },
    /// Footnote definition [^label]: content
    FootnoteDefinition { label: String, index: usize, content: Vec<InlineRun> },
}

/// Result of parsing markdown, containing main content blocks and footnote definitions
#[derive(Clone, Debug)]
pub struct ParsedDocument {
    pub blocks: Vec<Block>,
    pub footnotes: Vec<Block>,
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

pub fn render_blocks(source: &str) -> ParsedDocument {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
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
    
    // Footnote tracking
    // Maps footnote labels to their display index (1-based, order of first reference)
    let mut footnote_indices: HashMap<String, usize> = HashMap::new();
    let mut next_footnote_index: usize = 1;
    // Collect footnote definitions: (label, content runs)
    let mut footnote_definitions: HashMap<String, Vec<InlineRun>> = HashMap::new();
    // Current footnote definition being parsed: Some(label) if inside a definition
    let mut current_footnote_def: Option<String> = None;
    // Runs for current footnote definition
    let mut footnote_runs: Vec<InlineRun> = Vec::new();

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
                // Don't clear runs if inside a footnote definition
                if current_footnote_def.is_none() {
                    runs.clear();
                    heading_level = None;
                    in_list_item = false;
                }
            }
            Event::End(TagEnd::Paragraph) => {
                // Don't push blocks if inside a footnote definition
                if current_footnote_def.is_none() {
                    let kind = if in_quote {
                        BlockKind::Quote
                    } else {
                        BlockKind::Paragraph
                    };
                    push_runs_as(&mut blocks, &mut runs, kind);
                    bold_stack = 0;
                    italic_stack = 0;
                }
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
            // Footnote definition start
            Event::Start(Tag::FootnoteDefinition(label)) => {
                current_footnote_def = Some(label.to_string());
                footnote_runs.clear();
            }
            // Footnote definition end
            Event::End(TagEnd::FootnoteDefinition) => {
                if let Some(label) = current_footnote_def.take() {
                    footnote_definitions.insert(label, footnote_runs.clone());
                    footnote_runs.clear();
                }
            }
            // Footnote reference [^label]
            Event::FootnoteReference(label) => {
                let label_str = label.to_string();
                // Assign an index if this is the first reference to this footnote
                let index = *footnote_indices
                    .entry(label_str.clone())
                    .or_insert_with(|| {
                        let idx = next_footnote_index;
                        next_footnote_index += 1;
                        idx
                    });
                
                // Push current runs as a paragraph if there are any, then add the footnote ref
                if !runs.is_empty() {
                    let kind = if in_quote {
                        BlockKind::Quote
                    } else {
                        BlockKind::Paragraph
                    };
                    push_runs_as(&mut blocks, &mut runs, kind);
                }
                blocks.push(Block::FootnoteRef {
                    label: label_str,
                    index,
                });
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
                // If inside a footnote definition, add to footnote_runs
                if current_footnote_def.is_some() {
                    footnote_runs.push(InlineRun::new(
                        t.to_string(),
                        bold_stack > 0,
                        italic_stack > 0,
                        false,
                        link_stack.last().cloned(),
                    ));
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
                // If inside a footnote definition, add to footnote_runs
                if current_footnote_def.is_some() {
                    footnote_runs.push(InlineRun::new(
                        t.to_string(),
                        bold_stack > 0,
                        italic_stack > 0,
                        true,
                        link_stack.last().cloned(),
                    ));
                    continue;
                }
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
                // If inside a footnote definition, add to footnote_runs
                if current_footnote_def.is_some() {
                    footnote_runs.push(InlineRun::new(
                        "\n".to_string(),
                        bold_stack > 0,
                        italic_stack > 0,
                        false,
                        link_stack.last().cloned(),
                    ));
                    continue;
                }
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

    // Build footnote definitions list, ordered by index
    let mut footnotes: Vec<(usize, String, Vec<InlineRun>)> = footnote_indices
        .iter()
        .filter_map(|(label, &index)| {
            footnote_definitions
                .remove(label)
                .map(|content| (index, label.clone(), content))
        })
        .collect();
    footnotes.sort_by_key(|(index, _, _)| *index);
    
    let footnote_blocks: Vec<Block> = footnotes
        .into_iter()
        .map(|(index, label, content)| Block::FootnoteDefinition {
            label,
            index,
            content,
        })
        .collect();

    ParsedDocument {
        blocks,
        footnotes: footnote_blocks,
    }
}

enum BlockKind {
    Paragraph,
    Heading(u32),
    ListItem,
    Quote,
}

