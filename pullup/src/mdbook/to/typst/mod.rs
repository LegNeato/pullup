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
    chapters: VecDeque<()>,
    buf: VecDeque<ParserEvent<'a>>,
    iter: T,
}

impl<'a, T> ConvertChapter<'a, T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    #[allow(dead_code)]
    fn new(iter: T) -> Self {
        Self {
            chapters: VecDeque::new(),
            buf: VecDeque::new(),
            iter,
        }
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
                _,
                _,
            )))) => {
                #[cfg(feature = "tracing")]
                tracing::trace!("chapter start: {}", name);

                // Get how many chapters deep we are.
                let depth = self.chapters.len();

                // Create a Typst heading start event for the chapter.
                let tag = typst::Tag::Heading(
                    NonZeroU8::new((1 + depth).try_into().expect("nonzero")).expect("nonzero"),
                    typst::TableOfContents::Include,
                    typst::Bookmarks::Include,
                );

                let start_event = ParserEvent::Typst(typst::Event::Start(tag.clone()));
                let end_event = ParserEvent::Typst(typst::Event::End(tag));

                // Queue up the chapter name text event and heading end event.
                self.buf
                    .push_back(ParserEvent::Typst(typst::Event::Text(name.clone())));
                self.buf.push_back(end_event);

                // Record that we are one chapter deeper.
                self.chapters.push_back(());

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
                let _ = self.chapters.pop_front();
                // Chapters are converted to page break.
                Some(ParserEvent::Typst(typst::Event::FunctionCall(
                    None,
                    "pagebreak".into(),
                    vec!["weak: true".into()],
                )))
            }
            // Heading start in a chapter.
            Some(ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(
                markdown::Event::Start(markdown::Tag::Heading(level, x, y)),
            ))) => {
                #[cfg(feature = "tracing")]
                tracing::trace!("heading start: {}", level);

                use markdown::HeadingLevel;
                // Get how many chapters deep we are.
                let depth = self.chapters.len();
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
                        markdown::Event::Start(markdown::Tag::Heading(new_level, x, y)),
                    )));
                self.next()
            }
            // Heading end in a chapter.
            Some(ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(
                markdown::Event::End(markdown::Tag::Heading(level, x, y)),
            ))) => {
                #[cfg(feature = "tracing")]
                tracing::trace!("heading end: {}", level);
                use markdown::HeadingLevel;
                // Get how many chapters deep we are.
                let depth = self.chapters.len();
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
                        markdown::Event::End(markdown::Tag::Heading(new_level, x, y)),
                    )));
                self.next()
            }

            x => x,
        }
    }
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
