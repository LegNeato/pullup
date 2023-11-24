//! Convert mdBook to Typst.

use crate::converter;
use crate::markdown::CowStr;
use crate::mdbook;
use crate::typst;
use crate::ParserEvent;

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
    |iter: &mut I| {
        match iter.next() {
            Some(ParserEvent::Mdbook(mdbook::Event::Title(title))) => {
                Some(ParserEvent::Typst(typst::Event::DocumentSet(
                    "title".into(),
                    format!("\"{}\"", title.as_ref()).into()),
                ))
            },
            x => x,
    }
});

// TODO: tests
converter!(
    /// Convert mdBook chapters to Typst pagebreaks.
    ConvertChapter,
    ParserEvent<'a> => ParserEvent<'a>,
    |iter: &mut I| {
        match iter.next() {
            Some(ParserEvent::Mdbook(mdbook::Event::Start(mdbook::Tag::Chapter(_, _, _, _)))) => iter.next(),
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
