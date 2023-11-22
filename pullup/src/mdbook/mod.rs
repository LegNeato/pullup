//! Support for [mdBook](https://github.com/rust-lang/mdBook).

use crate::ParserEvent;
pub use pulldown_mdbook::{Event, Tag};

pub mod to;

/// Assert that an iterator only contains Mdbook events. Panics if another type of event
/// is encountered.
///
/// For a non-panic version, see [`MdbookFilter`].
pub struct AssertMdbook<T>(pub T);
impl<'a, T> Iterator for AssertMdbook<T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    type Item = self::Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.next() {
            None => None,
            Some(ParserEvent::Mdbook(x)) => Some(x),
            #[cfg(feature = "markdown")]
            Some(ParserEvent::Markdown(x)) => panic!("unexpected markdown event: {x:?}"),
            #[cfg(feature = "typst")]
            Some(ParserEvent::Typst(x)) => panic!("unexpected typst event: {x:?}"),
        }
    }
}

/// An iterator that only contains Mdbook events. Other types of events will be
/// filtered out.
///
/// To panic when a non-Mdbook event is encountered, see [`AssertMdbook`].
pub struct MdbookFilter<T>(pub T);
impl<'a, T> Iterator for MdbookFilter<T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.next() {
            None => None,
            Some(ParserEvent::Mdbook(x)) => Some(x),
            #[cfg(feature = "markdown")]
            Some(ParserEvent::Markdown(_)) => self.next(),
            #[cfg(feature = "typst")]
            Some(ParserEvent::Typst(_)) => self.next(),
        }
    }
}

/// An adaptor for events from an Mdbook parser.
pub struct MdbookIter<T>(pub T);

impl<'a, T> Iterator for MdbookIter<T>
where
    T: Iterator<Item = self::Event<'a>>,
{
    type Item = ParserEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(ParserEvent::Mdbook)
    }
}
