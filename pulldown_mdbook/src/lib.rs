use pulldown_cmark::CowStr;
use std::path::PathBuf;

pub mod markdown;
pub mod parser;

pub use parser::Parser;

use markdown::TextMergeStream;

#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    /// Start of a tagged element. Events that are yielded after this event
    /// and before its corresponding `End` event are inside this element.
    /// Start and end events are guaranteed to be balanced.
    Start(Tag<'a>),
    /// End of a tagged element.
    End(Tag<'a>),
    /// The root path of the book.
    Root(PathBuf),
    /// The title of the book.
    Title(CowStr<'a>),
    /// An author of the book.
    Author(CowStr<'a>),
    /// Separators can be added before, in-between, and after any other element.
    Separator,
    /// Parsed markdown content.
    MarkdownContentEvent(pulldown_cmark::Event<'a>),
}

/// Tags for elements that can contain other elements.
#[derive(Clone, Debug, PartialEq)]
pub enum Tag<'a> {
    /// A part is used to logically separate different sections of the book. The first
    /// field is the title. If the part is ordered the second field indicates the number
    /// of the first chapter.
    Part(Option<CowStr<'a>>, Option<u64>),

    /// A chapter represents book content. The first field indicates the status, the
    /// second field is the name, and the third field is the source. If the part is
    /// ordered the fourth field indicates the number of the chapter. Chapters can be
    /// nested.
    Chapter(
        ChapterStatus,
        CowStr<'a>,
        Option<ChapterSource<'a>>,
        Option<u64>,
    ),
    /// The content of the chapter.
    Content(ContentType),

    /// A list of the mdbook authors. Only contains Author events.
    AuthorList,
}

/// The status of a chapter.
#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ChapterStatus {
    Active,
    Draft,
}

/// The type of content.
#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ContentType {
    Markdown,
}

/// The source of a chapter.
#[derive(Clone, Debug, PartialEq)]
pub enum ChapterSource<'a> {
    Url(CowStr<'a>),
    Path(PathBuf),
}
