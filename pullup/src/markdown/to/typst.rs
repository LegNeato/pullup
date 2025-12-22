//! Convert Markdown to Typst.
use std::collections::VecDeque;

use crate::converter;
use crate::markdown;
use crate::typst;
use crate::ParserEvent;

converter!(
    /// Convert Markdown paragraphs to Typst paragraphs.
    ConvertParagraphs,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Paragraph))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph)))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::Paragraph))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph)))
            },
            x => x,
    }
});

/// Convert Markdown text to Typst text.
pub struct ConvertText<T> {
    code: VecDeque<()>,
    iter: T,
}

impl<'a, T> ConvertText<T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    pub fn new(iter: T) -> Self {
        ConvertText {
            code: VecDeque::new(),
            iter,
        }
    }
}

impl<'a, T> Iterator for ConvertText<T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    type Item = ParserEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.code.pop_back(), self.iter.next()) {
            // In code, include the unescaped text.
            (Some(_), Some(ParserEvent::Markdown(markdown::Event::Text(t)))) => {
                Some(ParserEvent::Typst(typst::Event::Text(t)))
            }
            // Not in code, escape the text using typist escaping rules.
            (None, Some(ParserEvent::Markdown(markdown::Event::Text(t)))) => {
                if t.trim().starts_with("\\[") && t.trim().ends_with("\\]") {
                    // Strip out mdbook's non-standard MathJax.
                    // TODO: Translate to typst math and/or expose this as a typed
                    // markdown event.
                    self.next()
                } else {
                    Some(ParserEvent::Typst(typst::Event::Text(t)))
                }
            }
            // Track code start.
            (
                _,
                event @ Some(ParserEvent::Markdown(markdown::Event::Start(
                    markdown::Tag::CodeBlock(_),
                ))),
            ) => {
                self.code.push_back(());
                event
            }
            // Track code end.
            (
                _,
                event @ Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::CodeBlock))),
            ) => {
                let _ = self.code.pop_back();
                event
            }
            (_, x) => x,
        }
    }
}

/// Convert mdBook internal links to Typst label references.
/// Only converts links that are clearly internal to the book:
/// - Paths with .md extension (book source files)
/// - Same-level or subdirectory paths with .html extension
/// - Links with anchors to sections within the book
///
/// E.g., "./chapter.md" -> "<chapter>"
///       "../other/section.md#anchor" -> "<anchor>"
///       "ch01-01-installation.html#troubleshooting" -> "<troubleshooting>"
///       "ch01-01-installation.html" -> "<ch01-01-installation>"
///       "../std/prelude/index.html" -> None (external to book)
fn md_path_to_label(url: &str) -> Option<String> {
    // Only handle relative paths, not external URLs
    if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("mailto:") {
        return None;
    }

    // Handle both .md and .html extensions (mdBook uses .html in rendered output)
    let has_md = url.contains(".md");
    let has_html = url.contains(".html");
    if !has_md && !has_html {
        return None;
    }

    // Reject ../ paths with .html extension (likely external docs like rustdoc)
    // But allow ../ paths with .md extension (internal book links)
    if url.starts_with("../") && !has_md {
        return None;
    }

    // Split into path and anchor parts
    let mut parts = url.splitn(2, '#');
    let path = parts.next().unwrap_or(url);
    let anchor = parts.next();

    // If there's an anchor, use that as the label (for section links within a page)
    // The heading converter will create labels from heading IDs
    if let Some(anchor) = anchor {
        if !anchor.is_empty() {
            return Some(format!("<{}>", anchor.to_lowercase()));
        }
    }

    // No anchor, use the path as label (for chapter links)
    // Strip leading ./ and ../ path components
    let label = path
        .trim_start_matches("./")
        .trim_start_matches("../")
        .trim_end_matches(".md")
        .trim_end_matches(".html")
        .replace('/', "-")
        .replace('\\', "-")
        .to_lowercase();

    if label.is_empty() {
        None
    } else {
        Some(format!("<{}>", label))
    }
}

converter!(
    /// Convert Markdown links to Typst links.
    ConvertLinks,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Link { link_type: kind, dest_url: url, .. }))) => {
                match kind {
                    markdown::LinkType::Inline
                    | markdown::LinkType::Reference
                    | markdown::LinkType::ReferenceUnknown
                    | markdown::LinkType::Collapsed
                    | markdown::LinkType::CollapsedUnknown
                    | markdown::LinkType::Shortcut
                    | markdown::LinkType::ShortcutUnknown
                    | markdown::LinkType::WikiLink { .. } => {
                        // Check if this is a relative .md link (intra-book link)
                        if let Some(label) = md_path_to_label(&url) {
                            Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Link(typst::LinkType::Content, label.into()))))
                        } else {
                            Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Link(typst::LinkType::Content, url))))
                        }
                    },
                    markdown::LinkType::Autolink => {
                        Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Link(typst::LinkType::Autolink, url))))
                    },
                    markdown::LinkType::Email => {
                        let url = "mailto:".to_string() + url.as_ref();
                        Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Link(typst::LinkType::Url, url.into()))))
                    },
                }
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::Link))) => {
                // End events don't have URL info in pulldown-cmark 0.13+, just emit generic end
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Link(typst::LinkType::Content, "".into()))))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown images to Typst images.
    ConvertImages,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Image { dest_url: url, title, .. }))) => {
                // Use title as alt text if available, otherwise empty
                let alt = if title.is_empty() { "".into() } else { title };
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Image(url, alt))))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::Image))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Image("".into(), "".into()))))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown **strong** tags to Typst strong tags.
    ConvertStrong,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Strong))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Strong)))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::Strong))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Strong)))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown _emphasis_ tags to Typst emphasis tags.
    ConvertEmphasis,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Emphasis))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Emphasis)))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::Emphasis))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Emphasis)))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown ~~strikethrough~~ tags to Typst strikethrough tags.
    ConvertStrikethrough,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Strikethrough))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Strikethrough)))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::Strikethrough))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Strikethrough)))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown soft breaks to Typst line breaks.
    ConvertSoftBreaks,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::SoftBreak)) => {
                Some(ParserEvent::Typst(typst::Event::Text(" ".into())))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown hard breaks to Typst line breaks.
    ConvertHardBreaks,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::HardBreak)) => {
                Some(ParserEvent::Typst(typst::Event::Linebreak))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown blockquotes to Typst quotes.
    ConvertBlockQuotes,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::BlockQuote(_)))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Quote(typst::QuoteType::Block, typst::QuoteQuotes::Auto, None))))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::BlockQuote(_)))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Quote(typst::QuoteType::Block, typst::QuoteQuotes::Auto, None))))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown code tags to Typst raw tags.
    ConvertCode,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            // Inline.
            Some(ParserEvent::Markdown(markdown::Event::Code(x))) => {
                Some(ParserEvent::Typst(typst::Event::Code(x)))
            },
            // Block.
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::CodeBlock(kind)))) => {
                match kind {
                    markdown::CodeBlockKind::Indented => Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::CodeBlock(None, typst::CodeBlockDisplay::Block)))),
                    markdown::CodeBlockKind::Fenced(val) => {
                        let val = if val.as_ref() == "" {
                            None
                        } else {
                            Some(val)
                        };
                        Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::CodeBlock(val, typst::CodeBlockDisplay::Block))))
                    },
                }
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::CodeBlock))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::CodeBlock(None, typst::CodeBlockDisplay::Block))))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown lists to Typst lists.
    ConvertLists,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        // TODO: Handle tight.

        // TODO: Allow changing the marker and number format.
        match this.iter.next() {
            // List start.
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::List(number)))) => {
                if let Some(start) = number {
                    // Numbered list
                    Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::NumberedList(start, None, false))))
                } else {
                    // Bullet list
                    Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::BulletList(None, false))))
                }

            },
            // List end.
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::List(ordered)))) => {
                if ordered {
                    // Numbered list
                    Some(ParserEvent::Typst(typst::Event::End(typst::Tag::NumberedList(1, None, false))))
                } else {
                    // Bullet list
                    Some(ParserEvent::Typst(typst::Event::End(typst::Tag::BulletList(None, false))))
                }

            },
            // List item start.
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Item))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Item)))
            },
            // List item end.
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::Item))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Item)))
            },
            x => x,
        }
   }
);

/// Generate a URL-friendly ID from text, matching mdBook's algorithm.
/// mdBook does NOT collapse multiple hyphens, so we preserve them.
fn generate_heading_id(text: &str) -> String {
    let result: String = text.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else if c == ' ' || c == '-' || c == '_' {
                '-'
            } else {
                // Skip other characters (punctuation, backticks, etc.)
                '\0'
            }
        })
        .filter(|&c| c != '\0')
        .collect();

    // Trim leading/trailing hyphens only
    result.trim_matches('-').to_string()
}

/// Buffered content for heading reconstruction.
/// Stores events that appear inside headings along with any text for ID generation.
#[derive(Clone)]
enum HeadingContent {
    /// Text content - also contributes to heading ID
    Text(String),
    /// Code content - also contributes to heading ID
    Code(String),
    /// Soft break
    SoftBreak,
    /// Hard break
    HardBreak,
    /// Strong emphasis start
    StrongStart,
    /// Strong emphasis end
    StrongEnd,
    /// Emphasis start
    EmphasisStart,
    /// Emphasis end
    EmphasisEnd,
}

/// Convert Markdown headings to Typst headings.
/// Generates heading IDs from text content for cross-referencing.
pub struct ConvertHeadings<T> {
    iter: T,
    /// Buffered heading content to emit
    buffer: VecDeque<HeadingContent>,
    /// Text collected for ID generation
    heading_text: String,
    /// Current heading level if inside a heading
    heading_level: Option<markdown::HeadingLevel>,
    /// Explicit ID from markdown if provided
    explicit_id: Option<String>,
    /// Whether we need to emit the heading start
    emit_start: Option<(std::num::NonZeroU8, Option<String>)>,
    /// Whether we need to emit the heading end (level, label)
    emit_end: Option<(std::num::NonZeroU8, Option<String>)>,
}

impl<'a, T> ConvertHeadings<T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    pub fn new(iter: T) -> Self {
        ConvertHeadings {
            iter,
            buffer: VecDeque::new(),
            heading_text: String::new(),
            heading_level: None,
            explicit_id: None,
            emit_start: None,
            emit_end: None,
        }
    }

    fn level_to_typst(level: markdown::HeadingLevel) -> std::num::NonZeroU8 {
        use markdown::HeadingLevel;
        match level {
            HeadingLevel::H1 => std::num::NonZeroU8::new(1).expect("non-zero"),
            HeadingLevel::H2 => std::num::NonZeroU8::new(2).expect("non-zero"),
            HeadingLevel::H3 => std::num::NonZeroU8::new(3).expect("non-zero"),
            HeadingLevel::H4 => std::num::NonZeroU8::new(4).expect("non-zero"),
            HeadingLevel::H5 => std::num::NonZeroU8::new(5).expect("non-zero"),
            HeadingLevel::H6 => std::num::NonZeroU8::new(6).expect("non-zero"),
        }
    }
}

impl<'a, T> Iterator for ConvertHeadings<T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    type Item = ParserEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Emit heading start if pending
        if let Some((level, label)) = self.emit_start.take() {
            return Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Heading(
                level,
                typst::TableOfContents::Include,
                typst::Bookmarks::Include,
                label.map(markdown::CowStr::from),
            ))));
        }

        // Emit buffered content - but only AFTER emit_start has been returned
        // (which means the heading End was processed and emit_end is set)
        if self.emit_end.is_some() {
            if let Some(content) = self.buffer.pop_front() {
                return Some(match content {
                    HeadingContent::Text(s) => ParserEvent::Markdown(markdown::Event::Text(markdown::CowStr::from(s))),
                    HeadingContent::Code(s) => ParserEvent::Markdown(markdown::Event::Code(markdown::CowStr::from(s))),
                    HeadingContent::SoftBreak => ParserEvent::Markdown(markdown::Event::SoftBreak),
                    HeadingContent::HardBreak => ParserEvent::Markdown(markdown::Event::HardBreak),
                    HeadingContent::StrongStart => ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Strong)),
                    HeadingContent::StrongEnd => ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::Strong)),
                    HeadingContent::EmphasisStart => ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Emphasis)),
                    HeadingContent::EmphasisEnd => ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::Emphasis)),
                });
            }
        }

        // Emit heading end if pending
        if let Some((level, label)) = self.emit_end.take() {
            return Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Heading(
                level,
                typst::TableOfContents::Include,
                typst::Bookmarks::Include,
                label.map(markdown::CowStr::from),
            ))));
        }

        match self.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Heading { level, id, .. }))) => {
                // Start collecting heading content
                self.heading_level = Some(level);
                self.heading_text.clear();
                self.explicit_id = id.map(|s| s.to_string());
                self.next() // Continue to collect content
            }
            Some(ParserEvent::Markdown(markdown::Event::Text(text))) if self.heading_level.is_some() => {
                // Collect text for ID generation and buffer it
                self.heading_text.push_str(&text);
                self.buffer.push_back(HeadingContent::Text(text.to_string()));
                self.next()
            }
            Some(ParserEvent::Markdown(markdown::Event::Code(code))) if self.heading_level.is_some() => {
                // Code in headings contributes to ID
                self.heading_text.push_str(&code);
                self.buffer.push_back(HeadingContent::Code(code.to_string()));
                self.next()
            }
            Some(ParserEvent::Markdown(markdown::Event::SoftBreak)) if self.heading_level.is_some() => {
                self.heading_text.push(' ');
                self.buffer.push_back(HeadingContent::SoftBreak);
                self.next()
            }
            Some(ParserEvent::Markdown(markdown::Event::HardBreak)) if self.heading_level.is_some() => {
                self.heading_text.push(' ');
                self.buffer.push_back(HeadingContent::HardBreak);
                self.next()
            }
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Strong))) if self.heading_level.is_some() => {
                self.buffer.push_back(HeadingContent::StrongStart);
                self.next()
            }
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::Strong))) if self.heading_level.is_some() => {
                self.buffer.push_back(HeadingContent::StrongEnd);
                self.next()
            }
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Emphasis))) if self.heading_level.is_some() => {
                self.buffer.push_back(HeadingContent::EmphasisStart);
                self.next()
            }
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::Emphasis))) if self.heading_level.is_some() => {
                self.buffer.push_back(HeadingContent::EmphasisEnd);
                self.next()
            }
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::Heading(level)))) => {
                let typst_level = Self::level_to_typst(level);
                // Generate ID from collected text or use explicit ID
                let id = self.explicit_id.take()
                    .unwrap_or_else(|| generate_heading_id(&self.heading_text));
                let label = if id.is_empty() { None } else { Some(id) };

                // Set up to emit: start, buffered content, end
                self.emit_start = Some((typst_level, label.clone()));
                self.emit_end = Some((typst_level, label));
                self.heading_level = None;
                self.heading_text.clear();

                self.next()
            }
            x => x,
        }
    }
}

converter!(
    /// Convert Markdown tables to Typst tables.
    ConvertTables,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            // Handle starting a table
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Table(alignment)))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Table(
                    alignment.iter().map(|&a| match a {
                        markdown::Alignment::Left => typst::TableCellAlignment::Left,
                        markdown::Alignment::Center => typst::TableCellAlignment::Center,
                        markdown::Alignment::Right => typst::TableCellAlignment::Right,
                        markdown::Alignment::None => typst::TableCellAlignment::None,
                    }).collect(),
                ))))
            },
            // Handle ending a table
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::Table))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Table(vec![]))))
            },
            // Handle header row
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::TableHead))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::TableHead)))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::TableHead))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::TableHead)))
            },
            // Handle starting a row
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::TableRow))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::TableRow)))
            },
            // Handle ending a row
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::TableRow))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::TableRow)))
            },
            // Handle starting a cell
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::TableCell))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::TableCell)))
            },
            // Handle ending a cell
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::TableCell))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::TableCell)))
            },
            // Pass through any other events
            x => x,
        }
    }
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::CowStr;
    use crate::markdown::{MarkdownIter, Parser};
    use similar_asserts::assert_eq;
    use std::num::NonZeroU8;

    // Set up type names so they are clearer and more succint.
    use markdown::Event as MdEvent;
    use markdown::HeadingLevel;
    use markdown::Tag as MdTag;
    use markdown::TagEnd as MdTagEnd;
    use typst::Event as TypstEvent;
    use typst::Tag as TypstTag;
    use ParserEvent::*;

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#atx-headings
    /// * https://spec.commonmark.org/0.30/#setext-headings Typst docs:
    /// * https://typst.app/docs/reference/meta/heading/
    mod headings {
        use super::*;

        #[test]
        fn convert_headings() {
            let md = "\
# Greetings

## This is **rad**!
";
            let i = ConvertHeadings::new(MarkdownIter(Parser::new(&md)));

            // ConvertHeadings now generates labels from heading text for cross-referencing
            // Strong/Emphasis events inside headings are preserved
            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::Heading(
                        NonZeroU8::new(1).unwrap(),
                        typst::TableOfContents::Include,
                        typst::Bookmarks::Include,
                        Some(CowStr::Boxed("greetings".into())),
                    ))),
                    Markdown(MdEvent::Text(CowStr::Boxed("Greetings".into()))),
                    Typst(TypstEvent::End(TypstTag::Heading(
                        NonZeroU8::new(1).unwrap(),
                        typst::TableOfContents::Include,
                        typst::Bookmarks::Include,
                        Some(CowStr::Boxed("greetings".into())),
                    ))),
                    Typst(TypstEvent::Start(TypstTag::Heading(
                        NonZeroU8::new(2).unwrap(),
                        typst::TableOfContents::Include,
                        typst::Bookmarks::Include,
                        Some(CowStr::Boxed("this-is-rad".into())),
                    ))),
                    Markdown(MdEvent::Text(CowStr::Boxed("This is ".into()))),
                    Markdown(MdEvent::Start(MdTag::Strong)),
                    Markdown(MdEvent::Text(CowStr::Boxed("rad".into()))),
                    Markdown(MdEvent::End(MdTagEnd::Strong)),
                    Markdown(MdEvent::Text(CowStr::Boxed("!".into()))),
                    Typst(TypstEvent::End(TypstTag::Heading(
                        NonZeroU8::new(2).unwrap(),
                        typst::TableOfContents::Include,
                        typst::Bookmarks::Include,
                        Some(CowStr::Boxed("this-is-rad".into())),
                    ))),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#link-reference-definitions
    /// * https://spec.commonmark.org/0.30/#links
    /// * https://spec.commonmark.org/0.30/#autolinks Typst docs:
    /// * https://typst.app/docs/reference/meta/link/
    mod links {
        use super::*;
        #[test]
        fn inline() {
            let md = "\
Cool [beans](https://example.com)
";
            let i = ConvertLinks::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Cool "))),
                    Typst(TypstEvent::Start(TypstTag::Link(
                        typst::LinkType::Content,
                        CowStr::Borrowed("https://example.com")
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("beans"))),
                    // Note: End events in pulldown-cmark 0.13 don't carry URL
                    Typst(TypstEvent::End(TypstTag::Link(
                        typst::LinkType::Content,
                        CowStr::Borrowed("")
                    ))),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                ]
            );
        }

        #[test]
        fn auto() {
            let md = "\
Cool <https://example.com>
";
            let i = ConvertLinks::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Cool "))),
                    Typst(TypstEvent::Start(TypstTag::Link(
                        typst::LinkType::Autolink,
                        CowStr::Borrowed("https://example.com")
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("https://example.com"))),
                    // Note: End events in pulldown-cmark 0.13 don't carry URL or type
                    Typst(TypstEvent::End(TypstTag::Link(
                        typst::LinkType::Content,
                        CowStr::Borrowed("")
                    ))),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                ]
            );
        }

        #[test]
        fn email() {
            let md = "\
Who are <you@example.com>
";
            let i = ConvertLinks::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Who are "))),
                    Typst(TypstEvent::Start(TypstTag::Link(
                        typst::LinkType::Url,
                        CowStr::Boxed("mailto:you@example.com".into())
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("you@example.com"))),
                    // Note: End events in pulldown-cmark 0.13 don't carry URL or type
                    Typst(TypstEvent::End(TypstTag::Link(
                        typst::LinkType::Content,
                        CowStr::Borrowed("")
                    ))),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                ]
            );
        }

        #[test]
        fn intra_book_link() {
            let md = "\
See [Chapter 2](./chapter2.md) for more info.
";
            let i = ConvertLinks::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("See "))),
                    Typst(TypstEvent::Start(TypstTag::Link(
                        typst::LinkType::Content,
                        CowStr::Boxed("<chapter2>".into())
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Chapter 2"))),
                    // Note: End events in pulldown-cmark 0.13 don't carry URL
                    Typst(TypstEvent::End(TypstTag::Link(
                        typst::LinkType::Content,
                        CowStr::Borrowed("")
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed(" for more info."))),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                ]
            );
        }

        #[test]
        fn nested_intra_book_link_with_anchor() {
            // When a link has an anchor (#anchor), we use the anchor as the label
            // This matches how mdBook section links work
            let md = "\
See [Other Section](../other/section.md#anchor) for details.
";
            let i = ConvertLinks::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("See "))),
                    Typst(TypstEvent::Start(TypstTag::Link(
                        typst::LinkType::Content,
                        CowStr::Boxed("<anchor>".into())
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Other Section"))),
                    // Note: End events in pulldown-cmark 0.13 don't carry URL
                    Typst(TypstEvent::End(TypstTag::Link(
                        typst::LinkType::Content,
                        CowStr::Borrowed("")
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed(" for details."))),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                ]
            );
        }

        #[test]
        fn nested_intra_book_link_without_anchor() {
            // When a link has no anchor, we use the path as the label
            let md = "\
See [Other Section](../other/section.md) for details.
";
            let i = ConvertLinks::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("See "))),
                    Typst(TypstEvent::Start(TypstTag::Link(
                        typst::LinkType::Content,
                        CowStr::Boxed("<other-section>".into())
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Other Section"))),
                    // Note: End events in pulldown-cmark 0.13 don't carry URL
                    Typst(TypstEvent::End(TypstTag::Link(
                        typst::LinkType::Content,
                        CowStr::Borrowed("")
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed(" for details."))),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                ]
            );
        }
    }

    mod images {
        use super::*;

        #[test]
        fn simple_image() {
            let md = "![alt text](image.png)";
            let i = ConvertImages::new(MarkdownIter(Parser::new(&md)));

            let events: Vec<_> = i.collect();
            // Check that we have an image start event
            assert!(events.iter().any(|e| matches!(
                e,
                Typst(TypstEvent::Start(TypstTag::Image(src, _))) if src.as_ref() == "image.png"
            )));
        }

        #[test]
        fn image_with_title() {
            let md = r#"![alt text](image.png "Image Title")"#;
            let i = ConvertImages::new(MarkdownIter(Parser::new(&md)));

            let events: Vec<_> = i.collect();
            // Check that we have an image with title
            assert!(events.iter().any(|e| matches!(
                e,
                Typst(TypstEvent::Start(TypstTag::Image(src, title)))
                    if src.as_ref() == "image.png" && title.as_ref() == "Image Title"
            )));
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#emphasis-and-strong-emphasis Typst docs:
    /// * https://typst.app/docs/reference/text/strong/
    mod strong {
        use super::*;
        #[test]
        fn convert_strong() {
            let md = "\
## **Foo**

I **love** cake!
";
            let i = ConvertStrong::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Heading {
                        level: HeadingLevel::H2,
                        id: None,
                        classes: vec![],
                        attrs: vec![],
                    })),
                    Typst(TypstEvent::Start(TypstTag::Strong)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Foo"))),
                    Typst(TypstEvent::End(TypstTag::Strong)),
                    Markdown(MdEvent::End(MdTagEnd::Heading(HeadingLevel::H2))),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("I "))),
                    Typst(TypstEvent::Start(TypstTag::Strong)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("love"))),
                    Typst(TypstEvent::End(TypstTag::Strong)),
                    Markdown(MdEvent::Text(CowStr::Borrowed(" cake!"))),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#emphasis-and-strong-emphasis Typst docs:
    /// * https://typst.app/docs/reference/text/emph/
    mod emphasis {
        use super::*;
        #[test]
        fn convert_emphasis() {
            let md = "\
## _Foo_

I *love* cake!
";
            let i = ConvertEmphasis::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Heading {
                        level: HeadingLevel::H2,
                        id: None,
                        classes: vec![],
                        attrs: vec![],
                    })),
                    Typst(TypstEvent::Start(TypstTag::Emphasis)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Foo"))),
                    Typst(TypstEvent::End(TypstTag::Emphasis)),
                    Markdown(MdEvent::End(MdTagEnd::Heading(HeadingLevel::H2))),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("I "))),
                    Typst(TypstEvent::Start(TypstTag::Emphasis)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("love"))),
                    Typst(TypstEvent::End(TypstTag::Emphasis)),
                    Markdown(MdEvent::Text(CowStr::Borrowed(" cake!"))),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                ]
            );
        }
    }

    mod strikethrough {
        use super::*;

        #[test]
        fn convert_strikethrough() {
            let md = "This is ~~deleted~~ text.";
            let i = ConvertStrikethrough::new(MarkdownIter(Parser::new_ext(
                &md,
                pulldown_cmark::Options::ENABLE_STRIKETHROUGH,
            )));

            let events: Vec<_> = i.collect();
            assert!(events.iter().any(|e| matches!(
                e,
                Typst(TypstEvent::Start(TypstTag::Strikethrough))
            )));
            assert!(events.iter().any(|e| matches!(
                e,
                Typst(TypstEvent::End(TypstTag::Strikethrough))
            )));
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#code Typst docs:
    /// * https://typst.app/docs/reference/text/raw/
    mod code {
        use super::*;
        #[test]
        fn inline() {
            let md = "\
foo `bar` baz
";
            let i = ConvertCode::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("foo "))),
                    Typst(TypstEvent::Code(CowStr::Borrowed("bar"))),
                    Markdown(MdEvent::Text(CowStr::Borrowed(" baz"))),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                ]
            );
        }

        #[test]
        fn block_indent() {
            let md = "\
whatever

    code 1
    code 2
";
            let i = ConvertCode::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("whatever"))),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                    Typst(TypstEvent::Start(TypstTag::CodeBlock(
                        None,
                        typst::CodeBlockDisplay::Block
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("code 1\n"))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("code 2\n"))),
                    Typst(TypstEvent::End(TypstTag::CodeBlock(
                        None,
                        typst::CodeBlockDisplay::Block
                    ))),
                ]
            );
        }

        #[test]
        fn block() {
            let md = "\
```
blah
```
";
            let i = ConvertCode::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::CodeBlock(
                        None,
                        typst::CodeBlockDisplay::Block
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("blah\n"))),
                    Typst(TypstEvent::End(TypstTag::CodeBlock(
                        None,
                        typst::CodeBlockDisplay::Block
                    ))),
                ]
            );
        }

        #[test]
        fn block_with_fence() {
            let md = "\
```foo
blah
```
";
            let i = ConvertCode::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::CodeBlock(
                        Some(CowStr::Borrowed("foo")),
                        typst::CodeBlockDisplay::Block
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("blah\n"))),
                    // Note: End events in pulldown-cmark 0.13 don't carry fence info
                    Typst(TypstEvent::End(TypstTag::CodeBlock(
                        None,
                        typst::CodeBlockDisplay::Block
                    ))),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#text Typst docs:
    /// * https://typst.app/docs/reference/text/
    mod text {
        use super::*;
        #[test]
        fn convert_text() {
            let md = "\
foo

bar

baz
";
            let i = ConvertText::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Typst(TypstEvent::Text(CowStr::Borrowed("foo"))),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Typst(TypstEvent::Text(CowStr::Borrowed("bar"))),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Typst(TypstEvent::Text(CowStr::Borrowed("baz"))),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.31.2/#hard-line-breaks
    /// * https://spec.commonmark.org/0.31.2/#soft-line-breaks
    /// Typst docs:
    /// * https://typst.app/docs/reference/text/
    mod breaks {
        use super::*;

        #[test]
        fn soft() {
            // Note that "foo" DOES NOT HAVE two spaces after it.
            let md = "\
foo
bar
";
            let i = ConvertSoftBreaks::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("foo"))),
                    Typst(TypstEvent::Text(CowStr::Borrowed(" "))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("bar"))),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                ]
            );
        }

        #[test]
        fn hard() {
            // Note that "foo" has two spaces after it.
            let md = "\
foo  
bar
";
            let i = ConvertHardBreaks::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("foo"))),
                    Typst(TypstEvent::Linebreak),
                    Markdown(MdEvent::Text(CowStr::Borrowed("bar"))),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#paragraphs Typst docs:
    /// * https://typst.app/docs/reference/layout/par/
    mod paragraphs {
        use super::*;
        #[test]
        fn convert_paragraphs() {
            let md = "\
foo

bar

baz
";
            let i = ConvertParagraphs::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("foo"))),
                    Typst(TypstEvent::End(TypstTag::Paragraph)),
                    Typst(TypstEvent::Start(TypstTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("bar"))),
                    Typst(TypstEvent::End(TypstTag::Paragraph)),
                    Typst(TypstEvent::Start(TypstTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("baz"))),
                    Typst(TypstEvent::End(TypstTag::Paragraph)),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#lists Typst docs:
    /// * https://typst.app/docs/reference/layout/list
    /// * https://typst.app/docs/reference/layout/enum/
    mod lists {
        use super::*;

        #[test]
        fn bullet() {
            let md = "\
* dogs
* are
* cool
";
            let i = ConvertLists::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::BulletList(None, false))),
                    // First bulet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("dogs"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    // Second bullet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("are"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    // Third bullet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("cool"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    Typst(TypstEvent::End(TypstTag::BulletList(None, false))),
                ],
            );
        }

        #[test]
        fn numbered() {
            let md = "\
1. cats are _too_
2. birds are ok
";
            let i = ConvertLists::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::NumberedList(1, None, false))),
                    // First bullet
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("cats are "))),
                    Markdown(MdEvent::Start(MdTag::Emphasis)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("too"))),
                    Markdown(MdEvent::End(MdTagEnd::Emphasis)),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    // Second bullet
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("birds are ok"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    Typst(TypstEvent::End(TypstTag::NumberedList(1, None, false))),
                ],
            );
        }

        #[test]
        fn numbered_custom_start() {
            let md = "\
6. foo
1. bar
";
            let i = ConvertLists::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::NumberedList(6, None, false))),
                    // First bullet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("foo"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    // Second bullet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("bar"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    // Note: End events in pulldown-cmark 0.13 don't carry custom start
                    Typst(TypstEvent::End(TypstTag::NumberedList(1, None, false))),
                ],
            );
        }

        #[test]
        fn multiple_lines() {
            let md = "\
* multiple
  lines
";
            let i = ConvertLists::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::BulletList(None, false))),
                    // First bullet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("multiple"))),
                    Markdown(MdEvent::SoftBreak),
                    Markdown(MdEvent::Text(CowStr::Borrowed("lines"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    Typst(TypstEvent::End(TypstTag::BulletList(None, false))),
                ]
            );
        }
    }

    mod issues {
        use super::*;

        // https://github.com/LegNeato/mdbook-typst/issues/3
        #[test]
        fn backslashes_in_backticks() {
            let md = r###"before `\` after"###;

            let i = ConvertText::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Typst(TypstEvent::Text("before ".into())),
                    Markdown(MdEvent::Code(r#"\"#.into())),
                    Typst(TypstEvent::Text(" after".into())),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                ],
            );
        }

        // https://github.com/LegNeato/mdbook-typst/issues/9
        #[test]
        fn simple_blockquote() {
            let md = "> test";

            let i = ConvertBlockQuotes::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::Quote(
                        typst::QuoteType::Block,
                        typst::QuoteQuotes::Auto,
                        None,
                    ))),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("test"))),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                    Typst(TypstEvent::End(TypstTag::Quote(
                        typst::QuoteType::Block,
                        typst::QuoteQuotes::Auto,
                        None,
                    ))),
                ],
            );
        }

        // https://github.com/LegNeato/mdbook-typst/issues/9
        #[test]
        fn complex_blockquote() {
            let md = "> one\n> two\n> three";

            let i = ConvertBlockQuotes::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::Quote(
                        typst::QuoteType::Block,
                        typst::QuoteQuotes::Auto,
                        None,
                    ))),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("one"))),
                    Markdown(MdEvent::SoftBreak),
                    Markdown(MdEvent::Text(CowStr::Borrowed("two"))),
                    Markdown(MdEvent::SoftBreak),
                    Markdown(MdEvent::Text(CowStr::Borrowed("three"))),
                    Markdown(MdEvent::End(MdTagEnd::Paragraph)),
                    Typst(TypstEvent::End(TypstTag::Quote(
                        typst::QuoteType::Block,
                        typst::QuoteQuotes::Auto,
                        None,
                    ))),
                ],
            );
        }
    }

    mod tables {
        use super::*;

        #[test]
        fn simple_table() {
            let md = "\
| Header1 | Header2 |
|---------|---------|
| Cell1   | Cell2   |
";
            let i = ConvertTables::new(MarkdownIter(Parser::new_ext(
                &md,
                pulldown_cmark::Options::ENABLE_TABLES,
            )));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::Table(vec![
                        typst::TableCellAlignment::None,
                        typst::TableCellAlignment::None,
                    ]))),
                    Typst(TypstEvent::Start(TypstTag::TableHead)),
                    Typst(TypstEvent::Start(TypstTag::TableCell)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Header1"))),
                    Typst(TypstEvent::End(TypstTag::TableCell)),
                    Typst(TypstEvent::Start(TypstTag::TableCell)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Header2"))),
                    Typst(TypstEvent::End(TypstTag::TableCell)),
                    Typst(TypstEvent::End(TypstTag::TableHead)),
                    Typst(TypstEvent::Start(TypstTag::TableRow)),
                    Typst(TypstEvent::Start(TypstTag::TableCell)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Cell1"))),
                    Typst(TypstEvent::End(TypstTag::TableCell)),
                    Typst(TypstEvent::Start(TypstTag::TableCell)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Cell2"))),
                    Typst(TypstEvent::End(TypstTag::TableCell)),
                    Typst(TypstEvent::End(TypstTag::TableRow)),
                    // Note: End events in pulldown-cmark 0.13 don't carry alignment info
                    Typst(TypstEvent::End(TypstTag::Table(vec![]))),
                ]
            );
        }

        #[test]
        fn table_with_alignment() {
            let md = "\
| Header1 | Header2 |
|:--------|:-------:|
| Cell1   | Cell2   |
";
            let i = ConvertTables::new(MarkdownIter(Parser::new_ext(
                &md,
                pulldown_cmark::Options::ENABLE_TABLES,
            )));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::Table(vec![
                        typst::TableCellAlignment::Left,
                        typst::TableCellAlignment::Center,
                    ]))),
                    Typst(TypstEvent::Start(TypstTag::TableHead)),
                    Typst(TypstEvent::Start(TypstTag::TableCell)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Header1"))),
                    Typst(TypstEvent::End(TypstTag::TableCell)),
                    Typst(TypstEvent::Start(TypstTag::TableCell)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Header2"))),
                    Typst(TypstEvent::End(TypstTag::TableCell)),
                    Typst(TypstEvent::End(TypstTag::TableHead)),
                    Typst(TypstEvent::Start(TypstTag::TableRow)),
                    Typst(TypstEvent::Start(TypstTag::TableCell)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Cell1"))),
                    Typst(TypstEvent::End(TypstTag::TableCell)),
                    Typst(TypstEvent::Start(TypstTag::TableCell)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Cell2"))),
                    Typst(TypstEvent::End(TypstTag::TableCell)),
                    Typst(TypstEvent::End(TypstTag::TableRow)),
                    // Note: End events in pulldown-cmark 0.13 don't carry alignment info
                    Typst(TypstEvent::End(TypstTag::Table(vec![]))),
                ]
            );
        }

        /// End-to-end test: markdown table -> typst markup
        #[test]
        fn table_to_typst_markup() {
            use crate::typst::TypstFilter;
            use pulldown_typst::markup::TypstMarkup;

            let md = "\
| Left | Center | Right |
|:-----|:------:|------:|
| A    | B      | C     |
";
            // Full conversion pipeline
            let parser = Parser::new_ext(&md, pulldown_cmark::Options::ENABLE_TABLES);
            let md_iter = MarkdownIter(parser);
            let converted = ConvertTables::new(ConvertText::new(md_iter));
            let typst_only = TypstFilter(converted);
            let output: String = TypstMarkup::new(typst_only).collect();

            // Verify the output contains valid Typst table structure
            assert!(output.contains("#table("));
            assert!(output.contains("columns: 3"));
            assert!(output.contains("align: (left, center, right)"));
            assert!(output.contains("table.header("));
            assert!(output.contains("[Left]"));
            assert!(output.contains("[Center]"));
            assert!(output.contains("[Right]"));
            assert!(output.contains("[A]"));
            assert!(output.contains("[B]"));
            assert!(output.contains("[C]"));
        }
    }
}
