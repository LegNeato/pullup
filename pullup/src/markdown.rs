//! Support for [Markdown](https://commonmark.org/).

use crate::ParserEvent;
pub use pulldown_cmark::{Event, Tag};

/// Assert that an iterator only contains Markdown events. Panics if another type of
/// event is encountered.
///
/// For a non-panic version, see [`MarkdownFilter`].
pub struct AssertMarkdown<T>(pub T);
impl<'a, T> Iterator for AssertMarkdown<T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    type Item = self::Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.next() {
            None => None,
            Some(ParserEvent::Markdown(x)) => Some(x),
            #[cfg(feature = "mdbook")]
            Some(ParserEvent::Mdbook(x)) => panic!("unexpected mdbook event: {x:?}"),
            #[cfg(feature = "typst")]
            Some(ParserEvent::Typst(x)) => panic!("unexpected typst event: {x:?}"),
        }
    }
}

/// An iterator that only contains Markdown events. Other types of events will be
/// filtered out.
///
/// To panic when a non-markdown event is encountered, see [`AssertMarkdown`].
pub struct MarkdownFilter<T>(pub T);
impl<'a, T> Iterator for MarkdownFilter<T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.next() {
            None => None,
            Some(ParserEvent::Markdown(x)) => Some(x),
            #[cfg(feature = "mdbook")]
            Some(ParserEvent::Mdbook(_)) => self.next(),
            #[cfg(feature = "typst")]
            Some(ParserEvent::Typst(_)) => self.next(),
        }
    }
}

/// An adaptor for events from a Markdown parser.
pub struct MarkdownIter<T>(pub T);

impl<'a, T> Iterator for MarkdownIter<T>
where
    T: Iterator<Item = self::Event<'a>>,
{
    type Item = ParserEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(ParserEvent::Markdown)
    }
}
