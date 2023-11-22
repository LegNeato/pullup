//! Convert mdBook to Typst.

use crate::converter;
use crate::mdbook;
use crate::typst;
use crate::ParserEvent;

#[cfg(feature = "builder")]
pub mod builder;

// TODO: tests
converter!(
    /// Convert mdbook authors to typst authors.
    ConvertAuthors,
    ParserEvent<'a> => ParserEvent<'a>,
    |iter: &mut I| {
        match iter.next() {
            Some(ParserEvent::Mdbook(mdbook::Event::Start(mdbook::Tag::AuthorList))) => {
                // Set the authors variable to an empty array.
                Some(ParserEvent::Typst(typst::Event::Let("mdbookauthors".into(), "()".into())))
            },
            Some(ParserEvent::Mdbook(mdbook::Event::Author(a))) => {
                // Append author to the array.
                Some(ParserEvent::Typst(typst::Event::FunctionCall(
                    Some("mdbookauthors".into()),
                    "push".into(),
                    vec![format!("\"{}\"", a).into()],
                )))
            },
            Some(ParserEvent::Mdbook(mdbook::Event::End(mdbook::Tag::AuthorList))) => {
                // Set document authors to the array.
                Some(ParserEvent::Typst(typst::Event::Set("document".into(), "author".into(), "mdbookauthors".into())))
            },
            x => x,
    }
});

// TODO: tests
converter!(
    /// Convert mdbook title to typst set document title event.
    ConvertTitle,
    ParserEvent<'a> => ParserEvent<'a>,
    |iter: &mut I| {
        match iter.next() {
            Some(ParserEvent::Mdbook(mdbook::Event::Title(title))) => {
                Some(ParserEvent::Typst(typst::Event::Set(
                    "document".into(),
                    "title".into(),
                    format!("\"{}\"", title.as_ref()).into()),
                ))
            },
            x => x,
    }
});

// TODO: tests
converter!(
    /// Convert mdbook chapters to typst pagebreaks.
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
