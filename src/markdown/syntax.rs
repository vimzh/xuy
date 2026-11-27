//! Turns recognized Markdown fence languages into escaped, themeable syntax spans.

use std::sync::LazyLock;

use pulldown_cmark::{CodeBlockKind, CowStr, Event};
use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::{SyntaxReference, SyntaxSet};

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

pub(super) fn render_code<'a>(kind: &CodeBlockKind<'_>, text: CowStr<'a>) -> Event<'a> {
    if let CodeBlockKind::Fenced(info) = kind
        && let Some(language) = info.split_whitespace().next()
        && let Some(syntax) = SYNTAXES.find_syntax_by_token(language)
    {
        match highlight(&text, syntax) {
            Ok(html) => return Event::Html(html.into()),
            Err(error) => eprintln!("Could not highlight {language} code: {error}"),
        }
    }
    Event::Text(text)
}

fn highlight(text: &str, syntax: &SyntaxReference) -> Result<String, syntect::Error> {
    let mut output = ClassedHTMLGenerator::new_with_class_style(
        syntax,
        &SYNTAXES,
        ClassStyle::SpacedPrefixed { prefix: "syntax-" },
    );
    for line in text.split_inclusive('\n') {
        if line.ends_with('\n') {
            output.parse_html_for_line_which_includes_newline(line)?;
        } else {
            output.parse_html_for_line_which_includes_newline(&format!("{line}\n"))?;
        }
    }
    let mut html = output.finalize();
    // Syntax parsing expects a newline even if the final source line had none.
    if !text.ends_with('\n')
        && let Some(newline) = html.rfind('\n')
    {
        html.remove(newline);
    }
    Ok(html)
}
