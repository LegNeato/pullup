use std::num::NonZeroU8;
pub mod markup;
// TODO: remove this.
use pulldown_cmark::CowStr;

#[derive(Debug, PartialEq, Clone)]
pub enum Event<'a> {
    /// Start of a tagged element. Events that are yielded after this event
    /// and before its corresponding `End` event are inside this element.
    /// Start and end events are guaranteed to be balanced.
    Start(Tag<'a>),
    /// End of a tagged element.
    End(Tag<'a>),
    /// A text node.
    Text(CowStr<'a>),
    /// An inline code node.
    Code(CowStr<'a>),
    /// A soft line break.
    Linebreak,
    /// A hard line break.
    Parbreak,
    /// A page break.
    PageBreak,
    /// A line. The first field is the start point, the second is the end point, the
    /// third field is the length, the fourth is the angle, and the fifth is the stroke.
    ///
    /// See <https://typst.app/docs/reference/visualize/line/>.
    // TODO: make this strongly typed.
    Line(
        // start
        Option<(CowStr<'a>, CowStr<'a>)>,
        // end
        Option<(CowStr<'a>, CowStr<'a>)>,
        // length
        Option<CowStr<'a>>,
        // angle
        Option<CowStr<'a>>,
        // stroke
        Option<CowStr<'a>>,
    ),
    /// A let binding. First argument is lhs, second is rhs.
    ///
    /// See <https://typst.app/docs/reference/scripting/#bindings>.
    Let(CowStr<'a>, CowStr<'a>),
    /// A function call. The first field is the target variable (without `#`), the
    /// second is the function name, and the third is a list of arguments.
    ///
    /// If calling `document()`, prefer [`DocumentFunctionCall`].
    // TODO: make this strongly typed.
    FunctionCall(Option<CowStr<'a>>, CowStr<'a>, Vec<CowStr<'a>>),
    /// A `document` function call. The field is the list of arguments.
    ///
    /// Prefer this over the more general `FunctionCall` as document calls must appear
    /// before any content.
    ///
    /// See <https://typst.app/docs/reference/meta/document>.
    // TODO: make this strongly typed.
    DocumentFunctionCall(Vec<CowStr<'a>>),
    /// A set rule.
    ///
    /// If setting document metadata, prefer [`DocumentSet`].
    ///
    /// See <https://typst.app/docs/reference/styling/#set-rules>.
    // TODO: make this a tag.
    Set(CowStr<'a>, CowStr<'a>, CowStr<'a>),
    /// A `document` set rule. The first field is the parameter name, the second is the
    /// parameter value.
    ///
    /// Prefer this over the more general [`Set`] as document set rules must appear
    /// before any content.
    ///
    /// See <https://typst.app/docs/reference/meta/document>.
    DocumentSet(CowStr<'a>, CowStr<'a>),

    /// Raw string data what will be bassed through directly to typst. Prefer using
    /// other strongly-typed rules.
    Raw(CowStr<'a>),
}

/// Tags for elements that can contain other elements.
#[derive(Clone, Debug, PartialEq)]
pub enum Tag<'a> {
    /// A paragraph of text and other inline elements.
    Paragraph,

    /// A show rule.
    ///
    /// See <https://typst.app/docs/reference/styling/#show-rules>.
    Show(
        ShowType,
        CowStr<'a>,
        Option<(CowStr<'a>, CowStr<'a>, CowStr<'a>)>,
        Option<CowStr<'a>>,
    ),

    /// A heading. The first field indicates the level of the heading, the second if it
    /// should be included in outline, and the third if it should be included in
    /// bookmarks.
    Heading(NonZeroU8, TableOfContents, Bookmarks),

    /// A code block. The first argument is the
    /// fenced value if it exists, the second is how it should be displayed.
    CodeBlock(Option<CowStr<'a>>, CodeBlockDisplay),

    /// A bullted list. The first field indicates the marker to use, the second is if
    /// tight is desired. Contains only list items.
    BulletList(Option<&'a str>, bool),
    /// A numbered / enumerated list (also called an _enum_ by typst). The first field
    /// indicates the starting number, the second is the [numbering
    /// pattern](https://typst.app/docs/reference/meta/numbering/), the third is if
    /// tight is desired. Contains only list items.
    ///
    /// See <https://typst.app/docs/reference/layout/enum/>.
    NumberedList(u64, Option<NumberingPattern<'a>>, bool),
    /// A list item.
    Item,
    /// A quote.
    /// The second argument determines if it should be wrapped in quotes.
    /// The third argument is the attribution value if it exists.
    ///
    /// See <https://typst.app/docs/reference/model/quote/>.
    Quote(QuoteType, QuoteQuotes, Option<CowStr<'a>>),
    // Span-level tags
    Emphasis,
    Strong,
    Strikethrough,

    /// A link. The first field is the type and the second is the destination URL.
    Link(LinkType, CowStr<'a>),

    /// A table. The first field is the alignment of each column.
    Table(Vec<TableCellAlignment>),
    /// A table header row. Must come after a #[Tag::Table].
    TableHead,
    /// A table row. Must come after a #[Tag::Table].
    TableRow,
    /// A table row. Must come after a #[Tag::TableRow].
    TableCell,
}

/// How to display a code block.
#[derive(Clone, Debug, PartialEq)]
pub enum CodeBlockDisplay {
    Block,
    Inline,
}

/// Item appearance in bookmarks.
#[derive(Clone, Debug, PartialEq)]
pub enum Bookmarks {
    Include,
    Exclude,
}

/// Item appearance in the table of contents.
#[derive(Clone, Debug, PartialEq)]
pub enum TableOfContents {
    Include,
    Exclude,
}

/// The pattern to use whren numbering items.
///
/// See <https://typst.app/docs/reference/meta/numbering/>.
#[derive(Clone, Debug, PartialEq)]
pub struct NumberingPattern<'a>(&'a str);

/// Type specifier for Show rules. See [Tag::Show](enum.Tag.html#variant.Show) for
/// more information.
// TODO: support different dests.
#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ShowType {
    ShowSet,
    Function,
}

/// Type specifier for inline links. See [Tag::Link](enum.Tag.html#variant.Link) for
/// more information.
#[derive(Clone, Debug, PartialEq, Copy)]
pub enum LinkType {
    /// Link like `#link("https://example.com")`
    Url,
    /// Link like `#link("https://example.com")[my cool content]`
    Content,
    /// Autolink like `http://foo.bar/baz`.
    Autolink,
}

/// Type specifier for a quote.
#[derive(Clone, Debug, PartialEq, Copy)]
pub enum QuoteType {
    Block,
    Inline,
}

/// Include a quote in quotes.
/// See <https://typst.app/docs/reference/model/quote/#parameters-quotes>
#[derive(Clone, Debug, PartialEq, Copy)]
pub enum QuoteQuotes {
    WrapInDoubleQuotes,
    DoNotWrapInDoubleQuotes,
    Auto,
}

/// Alignment of a table cell.
#[derive(Clone, Debug, PartialEq, Copy)]
pub enum TableCellAlignment {
    Left,
    Center,
    Right,
    None,
}
