//! Support for [Typist](https://typst.app/docs).

pub use pulldown_typst::{Event, Tag};

use crate::ParserEvent;

/// Assert that an iterator only contains Typst events. Panics if another type of event
/// is encountered.
///
/// For a non-panic version, see [`TypstFilter`].
pub struct AssertTypst<T>(pub T);
impl<'a, T> Iterator for AssertTypst<T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    type Item = self::Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.next() {
            None => None,
            Some(ParserEvent::Typst(x)) => Some(x),
            #[cfg(feature = "markdown")]
            Some(ParserEvent::Markdown(x)) => panic!("unexpected markdown event: {x:?}"),
            #[cfg(feature = "mdbook")]
            Some(ParserEvent::Mdbook(x)) => panic!("unexpected mdbook event: {x:?}"),
        }
    }
}

/// An iterator that only contains Typst events. Other types of events will be filtered
/// out.
///
/// To panic when a non-Typst event is encountered, see [`AssertTypst`].
pub struct TypstFilter<T>(pub T);
impl<'a, T> Iterator for TypstFilter<T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.next() {
            None => None,
            Some(ParserEvent::Typst(x)) => Some(x),
            #[cfg(feature = "markdown")]
            Some(ParserEvent::Markdown(_)) => self.next(),
            #[cfg(feature = "mdbook")]
            Some(ParserEvent::Mdbook(_)) => self.next(),
        }
    }
}

/// An adaptor for events from a Typst parser.
pub struct TypstIter<T>(pub T);

impl<'a, T> Iterator for TypstIter<T>
where
    T: Iterator<Item = self::Event<'a>>,
{
    type Item = ParserEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(ParserEvent::Typst)
    }
}
