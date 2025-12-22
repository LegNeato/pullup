//! Convert mdBook to Typst.

use crate::converter;
use crate::markdown;
use crate::markdown::CowStr;
use crate::mdbook;
use crate::typst;
use crate::ParserEvent;

use core::cmp::min;
use core::num::NonZeroU8;
use std::collections::VecDeque;

#[cfg(feature = "builder")]
mod builder;

#[cfg(feature = "builder")]
pub use builder::Conversion;

/// Convert mdBook authors to Typst authors.
#[derive(Debug)]
pub struct ConvertAuthors<'a, T> {
    authors: Vec<CowStr<'a>>,
    iter: T,
}

impl<'a, T> ConvertAuthors<'a, T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    #[allow(dead_code)]
    fn new(iter: T) -> Self {
        Self {
            authors: vec![],
            iter,
        }
    }
}

impl<'a, T> Iterator for ConvertAuthors<'a, T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    type Item = ParserEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(ParserEvent::Mdbook(mdbook::Event::Start(mdbook::Tag::AuthorList))) => {
                self.authors = vec![];
                self.next()
            }
            Some(ParserEvent::Mdbook(mdbook::Event::Author(a))) => {
                self.authors.push(a);
                self.next()
            }
            Some(ParserEvent::Mdbook(mdbook::Event::End(mdbook::Tag::AuthorList))) => {
                if !self.authors.is_empty() {
                    let markup_array = format!(
                        "({})",
                        // TODO: use intersperse once stable.
                        self.authors
                            .iter()
                            .map(|x| format!("\"{}\"", x))
                            .collect::<Vec<_>>()
                            .join(",")
                    );
                    return Some(ParserEvent::Typst(typst::Event::DocumentSet(
                        "author".into(),
                        markup_array.into(),
                    )));
                }
                self.next()
            }
            x => x,
        }
    }
}

// TODO: tests
converter!(
    /// Convert mdBook title to Typst set document title event.
    ConvertTitle,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Mdbook(mdbook::Event::Title(title))) => {
                Some(ParserEvent::Typst(typst::Event::DocumentSet(
                    "title".into(),
                    format!("\"{}\"", title.as_ref()).into()),
                ))
            },
            x => x,
    }
});

#[derive(Debug)]
pub struct ConvertChapter<'a, T> {
    /// Stack of chapter info for tracking nesting depth and current chapter path.
    /// Each entry is (label, source_path) where label is derived from the source path.
    chapter_info: VecDeque<Option<(String, std::path::PathBuf)>>,
    buf: VecDeque<ParserEvent<'a>>,
    iter: T,
}

/// Generate a label from a chapter source path.
/// Removes .md extension, replaces path separators with hyphens, and lowercases.
fn chapter_path_to_label(path: &std::path::Path) -> String {
    path.to_string_lossy()
        .trim_end_matches(".md")
        .replace(['/', '\\'], "-")
        .to_lowercase()
}

impl<'a, T> ConvertChapter<'a, T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    #[allow(dead_code)]
    fn new(iter: T) -> Self {
        Self {
            chapter_info: VecDeque::new(),
            buf: VecDeque::new(),
            iter,
        }
    }

    /// Get the current chapter label, if any.
    fn current_chapter_label(&self) -> Option<&str> {
        self.chapter_info
            .back()
            .and_then(|info| info.as_ref().map(|(label, _)| label.as_str()))
    }

    /// Get the current chapter source path, if any.
    fn current_chapter_path(&self) -> Option<&std::path::Path> {
        self.chapter_info
            .back()
            .and_then(|info| info.as_ref().map(|(_, path)| path.as_path()))
    }

    /// Resolve a relative URL against the current chapter's directory.
    /// Returns the resolved path if the URL is a relative .md file link,
    /// or None if it's an external URL or non-.md link.
    fn resolve_relative_url(&self, url_str: &str) -> Option<String> {
        // Use the url crate to detect absolute URLs
        // If parsing succeeds, it's an absolute URL (has a scheme) - skip it
        if url::Url::parse(url_str).is_ok() {
            return None;
        }

        // Skip same-page anchors (these are handled separately)
        if url_str.starts_with('#') {
            return None;
        }

        // Only handle .md files (internal book links)
        if !url_str.contains(".md") {
            return None;
        }

        let current_path = self.current_chapter_path()?;
        let current_dir = current_path.parent().unwrap_or(std::path::Path::new(""));

        // Split URL into path and anchor parts
        let (path_part, anchor_part) = if let Some(hash_pos) = url_str.find('#') {
            (&url_str[..hash_pos], Some(&url_str[hash_pos..]))
        } else {
            (url_str, None)
        };

        // Parse the relative path
        let relative_path = std::path::Path::new(path_part);

        // Resolve the path relative to current directory
        let mut resolved = current_dir.to_path_buf();
        for component in relative_path.components() {
            use std::path::Component;
            match component {
                Component::ParentDir => {
                    resolved.pop();
                }
                Component::CurDir => {
                    // Skip ./ components
                }
                Component::Normal(name) => {
                    resolved.push(name);
                }
                _ => {}
            }
        }

        // Convert resolved path back to string, adding anchor if present
        let mut result = resolved.to_string_lossy().to_string();
        if let Some(anchor) = anchor_part {
            result.push_str(anchor);
        }

        Some(result)
    }
}

impl<'a, T> Iterator for ConvertChapter<'a, T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    type Item = ParserEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // If we have a previously buffered event, return it.
        if let Some(buffered) = self.buf.pop_front() {
            #[cfg(feature = "tracing")]
            tracing::trace!("returning buffered: {:?}", buffered);

            return Some(buffered);
        }
        // Otherwise pull from the inner iterator.
        match self.iter.next() {
            // Start of chapter.
            Some(ParserEvent::Mdbook(mdbook::Event::Start(mdbook::Tag::Chapter(
                _,
                name,
                ref source,
                _,
            )))) => {
                #[cfg(feature = "tracing")]
                tracing::trace!("chapter start: {}", name);

                // Get how many chapters deep we are.
                let depth = self.chapter_info.len();

                // Generate a label and store source path for cross-references.
                let chapter_info: Option<(String, std::path::PathBuf)> =
                    source.as_ref().and_then(|s| match s {
                        mdbook::ChapterSource::Path(path) => {
                            Some((chapter_path_to_label(path), path.clone()))
                        }
                        mdbook::ChapterSource::Url(_) => None,
                    });
                let label = chapter_info.as_ref().map(|(l, _)| l.clone());

                // Create a Typst heading start event for the chapter.
                let tag = typst::Tag::Heading(
                    NonZeroU8::new((1 + depth).try_into().expect("nonzero")).expect("nonzero"),
                    typst::TableOfContents::Include,
                    typst::Bookmarks::Include,
                    label.clone().map(CowStr::from),
                );

                let start_event = ParserEvent::Typst(typst::Event::Start(tag.clone()));
                let end_event = ParserEvent::Typst(typst::Event::End(tag));

                // Queue up the chapter name text event and heading end event.
                self.buf
                    .push_back(ParserEvent::Typst(typst::Event::Text(name.clone())));
                self.buf.push_back(end_event);

                // Record that we are one chapter deeper, storing the label and path.
                self.chapter_info.push_back(chapter_info);

                #[cfg(feature = "tracing")]
                tracing::trace!("returning: {:?}", start_event);

                // Return the heading start event.
                Some(start_event)
            }
            // End of a chapter.
            Some(ParserEvent::Mdbook(mdbook::Event::End(mdbook::Tag::Chapter(_, _name, _, _)))) => {
                #[cfg(feature = "tracing")]
                tracing::trace!("chapter end: {}", _name);

                // Record that we are one chapter shallower.
                let _ = self.chapter_info.pop_back();
                // Chapters are converted to page break.
                Some(ParserEvent::Typst(typst::Event::FunctionCall(
                    None,
                    "pagebreak".into(),
                    vec!["weak: true".into()],
                )))
            }
            // Heading start in a chapter.
            Some(ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(
                markdown::Event::Start(markdown::Tag::Heading {
                    level,
                    id,
                    classes,
                    attrs,
                }),
            ))) => {
                #[cfg(feature = "tracing")]
                tracing::trace!("heading start: {}", level);

                use markdown::HeadingLevel;
                // Get how many chapters deep we are.
                let depth = self.chapter_info.len();
                let new_level = match level {
                    HeadingLevel::H1 => {
                        HeadingLevel::try_from(min(1 + depth, 6)).expect("valid heading level")
                    }
                    HeadingLevel::H2 => {
                        HeadingLevel::try_from(min(2 + depth, 6)).expect("valid heading level")
                    }
                    HeadingLevel::H3 => {
                        HeadingLevel::try_from(min(3 + depth, 6)).expect("valid heading level")
                    }
                    HeadingLevel::H4 => {
                        HeadingLevel::try_from(min(4 + depth, 6)).expect("valid heading level")
                    }
                    HeadingLevel::H5 => {
                        HeadingLevel::try_from(min(5 + depth, 6)).expect("valid heading level")
                    }
                    HeadingLevel::H6 => HeadingLevel::H6,
                };

                // Inject the chapter label as a prefix for heading IDs.
                // This ensures unique labels when multiple chapters have headings with the same text.
                // If the heading already has an ID (from mdBook or explicit {#id} syntax),
                // combine it with the chapter prefix and mark as complete with "!" prefix.
                // Otherwise, just set the chapter prefix (no marker) and ConvertHeadings
                // will append the generated ID from heading text.
                let new_id = match (self.current_chapter_label(), &id) {
                    (Some(chapter_label), Some(existing_id)) => {
                        // Mark as complete with "!" - ConvertHeadings will use as-is
                        Some(CowStr::from(format!(
                            "!{}-{}",
                            chapter_label,
                            existing_id.to_lowercase()
                        )))
                    }
                    (Some(chapter_label), None) => {
                        // Just the chapter prefix - ConvertHeadings will append generated ID
                        Some(CowStr::from(chapter_label.to_string()))
                    }
                    (None, _) => id.clone(),
                };

                self.buf
                    .push_back(ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(
                        markdown::Event::Start(markdown::Tag::Heading {
                            level: new_level,
                            id: new_id,
                            classes,
                            attrs,
                        }),
                    )));
                self.next()
            }
            // Heading end in a chapter.
            Some(ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(
                markdown::Event::End(markdown::TagEnd::Heading(level)),
            ))) => {
                #[cfg(feature = "tracing")]
                tracing::trace!("heading end: {}", level);
                use markdown::HeadingLevel;
                // Get how many chapters deep we are.
                let depth = self.chapter_info.len();
                let new_level = match level {
                    HeadingLevel::H1 => {
                        HeadingLevel::try_from(min(1 + depth, 6)).expect("valid heading level")
                    }
                    HeadingLevel::H2 => {
                        HeadingLevel::try_from(min(2 + depth, 6)).expect("valid heading level")
                    }
                    HeadingLevel::H3 => {
                        HeadingLevel::try_from(min(3 + depth, 6)).expect("valid heading level")
                    }
                    HeadingLevel::H4 => {
                        HeadingLevel::try_from(min(4 + depth, 6)).expect("valid heading level")
                    }
                    HeadingLevel::H5 => {
                        HeadingLevel::try_from(min(5 + depth, 6)).expect("valid heading level")
                    }
                    HeadingLevel::H6 => HeadingLevel::H6,
                };
                self.buf
                    .push_back(ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(
                        markdown::Event::End(markdown::TagEnd::Heading(new_level)),
                    )));
                self.next()
            }
            // Link start in a chapter - handle same-page anchors and resolve relative paths.
            Some(ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(
                markdown::Event::Start(markdown::Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    id,
                }),
            ))) => {
                let new_url = if let Some(anchor) = dest_url.strip_prefix('#') {
                    // Same-page anchor link - prefix the anchor with the chapter label
                    if let Some(chapter_label) = self.current_chapter_label() {
                        CowStr::from(format!("#{}-{}", chapter_label, anchor.to_lowercase()))
                    } else {
                        dest_url.clone()
                    }
                } else if let Some(resolved) = self.resolve_relative_url(&dest_url) {
                    // Cross-file relative link - resolve against current chapter directory
                    CowStr::from(resolved)
                } else {
                    dest_url.clone()
                };

                self.buf
                    .push_back(ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(
                        markdown::Event::Start(markdown::Tag::Link {
                            link_type,
                            dest_url: new_url,
                            title,
                            id,
                        }),
                    )));
                self.next()
            }
            // Handle inline HTML anchors like <a id="..."></a>
            // Convert them to Typst labels with chapter prefix
            Some(ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(
                markdown::Event::InlineHtml(html),
            ))) => {
                if let Some(anchor_id) = extract_html_anchor_id(&html) {
                    if let Some(chapter_label) = self.current_chapter_label() {
                        // Create a Typst label with chapter prefix
                        let label = format!("{}-{}", chapter_label, anchor_id.to_lowercase());
                        self.buf
                            .push_back(ParserEvent::Typst(typst::Event::Label(label.into())));
                        return self.next();
                    }
                }
                // Pass through other inline HTML
                Some(ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(
                    markdown::Event::InlineHtml(html),
                )))
            }
            // Handle block HTML anchors
            Some(ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(
                markdown::Event::Html(html),
            ))) => {
                if let Some(anchor_id) = extract_html_anchor_id(&html) {
                    if let Some(chapter_label) = self.current_chapter_label() {
                        // Create a Typst label with chapter prefix
                        let label = format!("{}-{}", chapter_label, anchor_id.to_lowercase());
                        self.buf
                            .push_back(ParserEvent::Typst(typst::Event::Label(label.into())));
                        return self.next();
                    }
                }
                // Pass through other HTML
                Some(ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(
                    markdown::Event::Html(html),
                )))
            }

            x => x,
        }
    }
}

/// Extract the ID from an HTML anchor element like `<a id="foo">` or `<a id="foo"></a>`.
/// Returns None if the HTML is not an anchor with an id attribute.
fn extract_html_anchor_id(html: &str) -> Option<String> {
    use html5ever::tokenizer::{
        BufferQueue, Tag, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
    };
    use std::cell::RefCell;

    struct AnchorIdExtractor {
        id: RefCell<Option<String>>,
    }

    impl TokenSink for AnchorIdExtractor {
        type Handle = ();

        fn process_token(&self, token: Token, _line_number: u64) -> TokenSinkResult<()> {
            if let Token::TagToken(Tag {
                kind: TagKind::StartTag,
                name,
                attrs,
                ..
            }) = token
            {
                // Check if this is an <a> tag
                if name.as_ref() == "a" {
                    // Look for the id attribute
                    for attr in attrs {
                        if attr.name.local.as_ref() == "id" {
                            *self.id.borrow_mut() = Some(attr.value.to_string());
                            break;
                        }
                    }
                }
            }
            TokenSinkResult::Continue
        }
    }

    let sink = AnchorIdExtractor {
        id: RefCell::new(None),
    };
    let tokenizer = Tokenizer::new(sink, TokenizerOpts::default());
    let buffer = BufferQueue::default();
    buffer.push_back(html.into());
    let _ = tokenizer.feed(&buffer);
    tokenizer.end();

    tokenizer.sink.id.into_inner()
}

// TODO: tests
converter!(
    /// Convert mdBook chapters to Typst pagebreaks. This does not affect any content in
    /// the chapter.
    ConvertChapterToPagebreak,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Mdbook(mdbook::Event::Start(mdbook::Tag::Chapter(_, _, _, _)))) => this.next(),
            Some(ParserEvent::Mdbook(mdbook::Event::End(mdbook::Tag::Chapter(_, _, _, _)))) => {
                Some(ParserEvent::Typst(typst::Event::FunctionCall(
                    None,
                    "pagebreak".into(),
                    vec!["weak: true".into()],
                )))
            },
            x => x,
    }
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::to::typst::ConvertHeadings;
    use crate::mdbook::MdbookIter;
    use std::path::PathBuf;

    /// Helper to create a chapter start event with a given path.
    fn chapter_start<'a>(name: &'a str, path: &str) -> mdbook::Event<'a> {
        mdbook::Event::Start(mdbook::Tag::Chapter(
            mdbook::ChapterStatus::Active,
            name.into(),
            Some(mdbook::ChapterSource::Path(PathBuf::from(path))),
            None,
        ))
    }

    /// Helper to create a chapter end event.
    fn chapter_end<'a>(name: &'a str) -> mdbook::Event<'a> {
        mdbook::Event::End(mdbook::Tag::Chapter(
            mdbook::ChapterStatus::Active,
            name.into(),
            None,
            None,
        ))
    }

    /// Helper to create a markdown heading start event within a chapter.
    fn heading_start<'a>(level: markdown::HeadingLevel) -> mdbook::Event<'a> {
        mdbook::Event::MarkdownContentEvent(markdown::Event::Start(markdown::Tag::Heading {
            level,
            id: None,
            classes: vec![],
            attrs: vec![],
        }))
    }

    /// Helper to create a markdown heading end event.
    fn heading_end(level: markdown::HeadingLevel) -> mdbook::Event<'static> {
        mdbook::Event::MarkdownContentEvent(markdown::Event::End(markdown::TagEnd::Heading(level)))
    }

    /// Helper to create a markdown text event within a chapter.
    fn text<'a>(s: &'a str) -> mdbook::Event<'a> {
        mdbook::Event::MarkdownContentEvent(markdown::Event::Text(s.into()))
    }

    #[test]
    fn chapter_path_to_label_simple() {
        let path = PathBuf::from("intro.md");
        assert_eq!(chapter_path_to_label(&path), "intro");
    }

    #[test]
    fn chapter_path_to_label_nested() {
        let path = PathBuf::from("chapters/getting-started.md");
        assert_eq!(chapter_path_to_label(&path), "chapters-getting-started");
    }

    #[test]
    fn heading_in_chapter_gets_prefixed_label() {
        // Create events: Chapter start -> Heading -> Text -> Heading end -> Chapter end
        let events = vec![
            chapter_start("Introduction", "intro.md"),
            heading_start(markdown::HeadingLevel::H2),
            text("Getting Started"),
            heading_end(markdown::HeadingLevel::H2),
            chapter_end("Introduction"),
        ];

        // Run through ConvertChapter then ConvertHeadings
        let iter = MdbookIter(events.into_iter());
        let converted = ConvertChapter::new(iter);

        // Map MarkdownContentEvent to Markdown for ConvertHeadings
        let mapped = converted.map(|e| match e {
            ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(m)) => ParserEvent::Markdown(m),
            x => x,
        });

        let with_headings = ConvertHeadings::new(mapped);
        let result: Vec<_> = with_headings.collect();

        // Find any heading event and check its label
        // Note: H2 in a chapter at depth 1 becomes H3 (2 + 1 = 3)
        let heading_label = result.iter().find_map(|e| match e {
            ParserEvent::Typst(typst::Event::Start(typst::Tag::Heading(
                level,
                _,
                _,
                Some(label),
            ))) if level.get() == 3 => Some(label.as_ref()),
            _ => None,
        });

        // The label should be "intro-getting-started" (chapter path prefix + heading text)
        assert_eq!(heading_label, Some("intro-getting-started"));
    }

    #[test]
    fn different_chapters_same_heading_get_unique_labels() {
        // Two chapters with the same "Introduction" heading
        let events = vec![
            chapter_start("Chapter 1", "chapter1.md"),
            heading_start(markdown::HeadingLevel::H2),
            text("Introduction"),
            heading_end(markdown::HeadingLevel::H2),
            chapter_end("Chapter 1"),
            chapter_start("Chapter 2", "chapter2.md"),
            heading_start(markdown::HeadingLevel::H2),
            text("Introduction"),
            heading_end(markdown::HeadingLevel::H2),
            chapter_end("Chapter 2"),
        ];

        let iter = MdbookIter(events.into_iter());
        let converted = ConvertChapter::new(iter);
        let mapped = converted.map(|e| match e {
            ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(m)) => ParserEvent::Markdown(m),
            x => x,
        });
        let with_headings = ConvertHeadings::new(mapped);
        let result: Vec<_> = with_headings.collect();

        // Collect all H3 heading labels (H2 in chapter at depth 1 becomes H3)
        let heading_labels: Vec<_> = result
            .iter()
            .filter_map(|e| match e {
                ParserEvent::Typst(typst::Event::Start(typst::Tag::Heading(
                    level,
                    _,
                    _,
                    Some(label),
                ))) if level.get() == 3 => Some(label.as_ref().to_string()),
                _ => None,
            })
            .collect();

        // Should have two unique labels
        assert_eq!(heading_labels.len(), 2);
        assert!(heading_labels.contains(&"chapter1-introduction".to_string()));
        assert!(heading_labels.contains(&"chapter2-introduction".to_string()));
    }

    #[test]
    fn nested_chapter_path() {
        let events = vec![
            chapter_start("Basics", "guides/basics.md"),
            heading_start(markdown::HeadingLevel::H2),
            text("Overview"),
            heading_end(markdown::HeadingLevel::H2),
            chapter_end("Basics"),
        ];

        let iter = MdbookIter(events.into_iter());
        let converted = ConvertChapter::new(iter);
        let mapped = converted.map(|e| match e {
            ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(m)) => ParserEvent::Markdown(m),
            x => x,
        });
        let with_headings = ConvertHeadings::new(mapped);
        let result: Vec<_> = with_headings.collect();

        // H2 in chapter at depth 1 becomes H3
        let heading_label = result.iter().find_map(|e| match e {
            ParserEvent::Typst(typst::Event::Start(typst::Tag::Heading(
                level,
                _,
                _,
                Some(label),
            ))) if level.get() == 3 => Some(label.as_ref()),
            _ => None,
        });

        // The label should include the nested path
        assert_eq!(heading_label, Some("guides-basics-overview"));
    }

    /// Helper to create a markdown link start event within a chapter.
    fn link_start<'a>(url: &'a str) -> mdbook::Event<'a> {
        mdbook::Event::MarkdownContentEvent(markdown::Event::Start(markdown::Tag::Link {
            link_type: markdown::LinkType::Inline,
            dest_url: url.into(),
            title: "".into(),
            id: "".into(),
        }))
    }

    /// Helper to create a markdown link end event.
    fn link_end() -> mdbook::Event<'static> {
        mdbook::Event::MarkdownContentEvent(markdown::Event::End(markdown::TagEnd::Link))
    }

    #[test]
    fn same_page_anchor_link_gets_prefixed() {
        // A same-page anchor link like #troubleshooting should be prefixed with chapter path
        let events = vec![
            chapter_start("Installation", "ch01-01-installation.md"),
            link_start("#troubleshooting"),
            text("Troubleshooting"),
            link_end(),
            chapter_end("Installation"),
        ];

        let iter = MdbookIter(events.into_iter());
        let converted = ConvertChapter::new(iter);
        let result: Vec<_> = converted.collect();

        // Find the link event and check its URL was prefixed
        let link_url = result.iter().find_map(|e| match e {
            ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(markdown::Event::Start(
                markdown::Tag::Link { dest_url, .. },
            ))) => Some(dest_url.as_ref()),
            _ => None,
        });

        // The anchor should be prefixed with chapter path
        assert_eq!(link_url, Some("#ch01-01-installation-troubleshooting"));
    }

    #[test]
    fn cross_page_anchor_link_combines_path_and_anchor() {
        use crate::markdown::to::typst::ConvertLinks;

        // A cross-page anchor link should combine the target chapter path with the anchor
        let md = "[See section](./ch02-02-advanced.md#configuration)";
        let parser = markdown::Parser::new(md);
        let i = ConvertLinks::new(crate::markdown::MarkdownIter(parser));
        let result: Vec<_> = i.collect();

        // Find the link event and check its URL
        let link_url = result.iter().find_map(|e| match e {
            ParserEvent::Typst(typst::Event::Start(typst::Tag::Link(_, url))) => Some(url.as_ref()),
            _ => None,
        });

        // The link should combine path and anchor
        assert_eq!(link_url, Some("<ch02-02-advanced-configuration>"));
    }

    /// Helper to create a markdown heading start event with an explicit ID.
    fn heading_start_with_id<'a>(level: markdown::HeadingLevel, id: &'a str) -> mdbook::Event<'a> {
        mdbook::Event::MarkdownContentEvent(markdown::Event::Start(markdown::Tag::Heading {
            level,
            id: Some(id.into()),
            classes: vec![],
            attrs: vec![],
        }))
    }

    #[test]
    fn heading_with_existing_id_preserves_id() {
        // When a heading has an explicit ID (from {#id} syntax or mdBook),
        // it should be used instead of generating from heading text
        let events = vec![
            chapter_start("Installation", "ch01-01-installation.md"),
            // This heading has an explicit short ID
            heading_start_with_id(markdown::HeadingLevel::H2, "troubleshooting"),
            text("Troubleshooting Guide for Common Issues"),
            heading_end(markdown::HeadingLevel::H2),
            chapter_end("Installation"),
        ];

        let iter = MdbookIter(events.into_iter());
        let converted = ConvertChapter::new(iter);
        let mapped = converted.map(|e| match e {
            ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(m)) => ParserEvent::Markdown(m),
            x => x,
        });
        let with_headings = ConvertHeadings::new(mapped);
        let result: Vec<_> = with_headings.collect();

        // Find the heading label
        let heading_label = result.iter().find_map(|e| match e {
            ParserEvent::Typst(typst::Event::Start(typst::Tag::Heading(
                level,
                _,
                _,
                Some(label),
            ))) if level.get() == 3 => Some(label.as_ref()),
            _ => None,
        });

        // The label should use the explicit ID, not the generated one from text
        // (which would be "troubleshooting-guide-for-common-issues")
        assert_eq!(heading_label, Some("ch01-01-installation-troubleshooting"));
    }

    #[test]
    fn extract_html_anchor_id_basic() {
        assert_eq!(
            super::extract_html_anchor_id(r#"<a id="foo"></a>"#).as_deref(),
            Some("foo")
        );
    }

    #[test]
    fn extract_html_anchor_id_single_quotes() {
        assert_eq!(
            super::extract_html_anchor_id(r#"<a id='bar'></a>"#).as_deref(),
            Some("bar")
        );
    }

    #[test]
    fn extract_html_anchor_id_with_other_attrs() {
        assert_eq!(
            super::extract_html_anchor_id(r##"<a class="link" id="baz" href="#"></a>"##).as_deref(),
            Some("baz")
        );
    }

    #[test]
    fn extract_html_anchor_id_not_anchor() {
        // html5ever parses any tag, so we only extract from <a> tags
        assert_eq!(
            super::extract_html_anchor_id(r#"<div id="foo"></div>"#),
            None
        );
        assert_eq!(
            super::extract_html_anchor_id(r#"<span id="foo"></span>"#),
            None
        );
    }

    #[test]
    fn extract_html_anchor_id_no_id() {
        assert_eq!(
            super::extract_html_anchor_id(r##"<a href="#foo"></a>"##),
            None
        );
    }

    /// Helper to create an inline HTML event.
    fn inline_html<'a>(html: &'a str) -> mdbook::Event<'a> {
        mdbook::Event::MarkdownContentEvent(markdown::Event::InlineHtml(html.into()))
    }

    #[test]
    fn html_anchor_gets_chapter_prefix() {
        // An HTML anchor like <a id="troubleshooting"></a> should become a Typst label
        // with the chapter prefix
        let events = vec![
            chapter_start("Installation", "ch01-01-installation.md"),
            inline_html(r#"<a id="troubleshooting"></a>"#),
            chapter_end("Installation"),
        ];

        let iter = MdbookIter(events.into_iter());
        let converted = ConvertChapter::new(iter);
        let result: Vec<_> = converted.collect();

        // Find the label event
        let label = result.iter().find_map(|e| match e {
            ParserEvent::Typst(typst::Event::Label(l)) => Some(l.as_ref()),
            _ => None,
        });

        // The label should have the chapter prefix
        assert_eq!(label, Some("ch01-01-installation-troubleshooting"));
    }

    #[test]
    fn html_anchor_matches_link_target() {
        use crate::markdown::to::typst::ConvertLinks;

        // Verify that an HTML anchor and a link to it will have matching labels
        // HTML anchor: <a id="troubleshooting"></a> in ch01-01-installation.md
        // Link: [text](ch01-01-installation.html#troubleshooting) from another chapter

        // First, check what label the HTML anchor produces
        let anchor_events = vec![
            chapter_start("Installation", "ch01-01-installation.md"),
            inline_html(r#"<a id="troubleshooting"></a>"#),
            chapter_end("Installation"),
        ];
        let iter = MdbookIter(anchor_events.into_iter());
        let converted = ConvertChapter::new(iter);
        let result: Vec<_> = converted.collect();
        let anchor_label = result.iter().find_map(|e| match e {
            ParserEvent::Typst(typst::Event::Label(l)) => Some(l.as_ref().to_string()),
            _ => None,
        });

        // Second, check what label the link produces
        let md = "[text](ch01-01-installation.html#troubleshooting)";
        let parser = markdown::Parser::new(md);
        let i = ConvertLinks::new(crate::markdown::MarkdownIter(parser));
        let result: Vec<_> = i.collect();
        let link_target = result.iter().find_map(|e| match e {
            ParserEvent::Typst(typst::Event::Start(typst::Tag::Link(_, url))) => {
                // Extract label name from <label-name>
                let s = url.as_ref();
                if s.starts_with('<') && s.ends_with('>') {
                    Some(s[1..s.len() - 1].to_string())
                } else {
                    None
                }
            }
            _ => None,
        });

        // Both should produce the same label
        assert_eq!(
            anchor_label,
            Some("ch01-01-installation-troubleshooting".to_string())
        );
        assert_eq!(
            link_target,
            Some("ch01-01-installation-troubleshooting".to_string())
        );
        assert_eq!(anchor_label, link_target);
    }

    #[test]
    fn relative_sibling_link_resolved() {
        // https://github.com/LegNeato/mdbook-typst/issues/21
        // When linking from about/index.md to about-this-handbook.md,
        // the link should resolve to about/about-this-handbook.md
        let events = vec![
            chapter_start("About", "about/index.md"),
            link_start("about-this-handbook.md"),
            text("About This Handbook"),
            link_end(),
            chapter_end("About"),
        ];

        let iter = MdbookIter(events.into_iter());
        let converted = ConvertChapter::new(iter);
        let result: Vec<_> = converted.collect();

        // Find the link event and check its URL was resolved
        let link_url = result.iter().find_map(|e| match e {
            ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(markdown::Event::Start(
                markdown::Tag::Link { dest_url, .. },
            ))) => Some(dest_url.as_ref().to_string()),
            _ => None,
        });

        // The relative link should be resolved to include the directory
        assert_eq!(link_url, Some("about/about-this-handbook.md".to_string()));
    }

    #[test]
    fn relative_parent_link_resolved() {
        // https://github.com/LegNeato/mdbook-typst/issues/23
        // When linking from installation/live-images.md to ../index.md,
        // the link should resolve to index.md (at root)
        let events = vec![
            chapter_start("Live Images", "installation/live-images.md"),
            link_start("../index.md"),
            text("Index"),
            link_end(),
            chapter_end("Live Images"),
        ];

        let iter = MdbookIter(events.into_iter());
        let converted = ConvertChapter::new(iter);
        let result: Vec<_> = converted.collect();

        let link_url = result.iter().find_map(|e| match e {
            ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(markdown::Event::Start(
                markdown::Tag::Link { dest_url, .. },
            ))) => Some(dest_url.as_ref().to_string()),
            _ => None,
        });

        // The parent-relative link should resolve correctly
        assert_eq!(link_url, Some("index.md".to_string()));
    }

    #[test]
    fn relative_link_with_anchor_resolved() {
        // https://github.com/LegNeato/mdbook-typst/issues/23
        // When linking from installation/live-images.md to ../index.md#downloading,
        // the link should resolve to index.md#downloading
        let events = vec![
            chapter_start("Live Images", "installation/live-images.md"),
            link_start("../index.md#downloading-installation-media"),
            text("downloading"),
            link_end(),
            chapter_end("Live Images"),
        ];

        let iter = MdbookIter(events.into_iter());
        let converted = ConvertChapter::new(iter);
        let result: Vec<_> = converted.collect();

        let link_url = result.iter().find_map(|e| match e {
            ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(markdown::Event::Start(
                markdown::Tag::Link { dest_url, .. },
            ))) => Some(dest_url.as_ref().to_string()),
            _ => None,
        });

        // The relative link with anchor should resolve correctly
        assert_eq!(
            link_url,
            Some("index.md#downloading-installation-media".to_string())
        );
    }

    #[test]
    fn deeply_nested_relative_link_resolved() {
        // From src/guides/installation/advanced.md linking to ../../about/index.md#getting-started
        let events = vec![
            chapter_start("Advanced", "src/guides/installation/advanced.md"),
            link_start("../../about/index.md#getting-started"),
            text("Getting Started"),
            link_end(),
            chapter_end("Advanced"),
        ];

        let iter = MdbookIter(events.into_iter());
        let converted = ConvertChapter::new(iter);
        let result: Vec<_> = converted.collect();

        let link_url = result.iter().find_map(|e| match e {
            ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(markdown::Event::Start(
                markdown::Tag::Link { dest_url, .. },
            ))) => Some(dest_url.as_ref().to_string()),
            _ => None,
        });

        // Multiple parent dir refs should be resolved correctly
        assert_eq!(
            link_url,
            Some("src/about/index.md#getting-started".to_string())
        );
    }

    #[test]
    fn external_link_not_modified() {
        // External URLs should pass through unchanged
        let events = vec![
            chapter_start("Installation", "installation.md"),
            link_start("https://example.com"),
            text("Example"),
            link_end(),
            chapter_end("Installation"),
        ];

        let iter = MdbookIter(events.into_iter());
        let converted = ConvertChapter::new(iter);
        let result: Vec<_> = converted.collect();

        let link_url = result.iter().find_map(|e| match e {
            ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(markdown::Event::Start(
                markdown::Tag::Link { dest_url, .. },
            ))) => Some(dest_url.as_ref().to_string()),
            _ => None,
        });

        assert_eq!(link_url, Some("https://example.com".to_string()));
    }

    #[test]
    fn dot_slash_prefix_resolved() {
        // ./about-this-handbook.md from about/index.md should resolve to about/about-this-handbook.md
        let events = vec![
            chapter_start("About", "about/index.md"),
            link_start("./about-this-handbook.md"),
            text("About This Handbook"),
            link_end(),
            chapter_end("About"),
        ];

        let iter = MdbookIter(events.into_iter());
        let converted = ConvertChapter::new(iter);
        let result: Vec<_> = converted.collect();

        let link_url = result.iter().find_map(|e| match e {
            ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(markdown::Event::Start(
                markdown::Tag::Link { dest_url, .. },
            ))) => Some(dest_url.as_ref().to_string()),
            _ => None,
        });

        // ./relative should work the same as plain relative
        assert_eq!(link_url, Some("about/about-this-handbook.md".to_string()));
    }
}
