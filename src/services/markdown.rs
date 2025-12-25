use crate::error::AppResult;
use pulldown_cmark::{html, Options, Parser};

pub fn render_html(source: &str) -> AppResult<String> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(source, options);
    let mut html_out = String::new();
    html::push_html(&mut html_out, parser);
    Ok(html_out)
}
