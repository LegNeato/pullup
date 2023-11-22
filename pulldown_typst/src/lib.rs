use std::{collections::VecDeque, fmt::Write, io::ErrorKind, num::NonZeroU8};

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
    /// A line.
    Line,
    /// A let binding. First argument is lhs, second is rhs.
    /// https://typst.app/docs/reference/scripting/#bindings
    Let(CowStr<'a>, CowStr<'a>),
    /// A function call. The first field is the target variable (without `#`), the
    /// second is the function name, and the third is a list of arguments.
    // TODO: make this strongly typed.
    FunctionCall(Option<CowStr<'a>>, CowStr<'a>, Vec<CowStr<'a>>),
    /// A set rule.
    /// https://typst.app/docs/reference/styling/#set-rules
    // TODO: make this a tag.
    Set(CowStr<'a>, CowStr<'a>, CowStr<'a>),
    /// Raw string data what will be bassed through directly to typst. Prefer using
    /// other strongly-typed rules.
    Raw(CowStr<'a>),
}

/// Tags for elements that can contain other elements.
#[derive(Clone, Debug, PartialEq)]
pub enum Tag<'a> {
    /// A paragraph of text and other inline elements.
    Paragraph,

    /// A show rule. https://typst.app/docs/reference/styling/#show-rules
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
    /// https://typst.app/docs/reference/layout/enum/
    NumberedList(u64, Option<NumberingPattern<'a>>, bool),
    /// A list item.
    Item,

    // Span-level tags
    Emphasis,
    Strong,
    Strikethrough,

    /// A link. The first field is the type and the second is the destination URL.
    Link(LinkType, CowStr<'a>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum CodeBlockDisplay {
    Inline,
    Block,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Bookmarks {
    Include,
    Exclude,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TableOfContents {
    Include,
    Exclude,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NumberingPattern<'a>(&'a str);

/// Type specifier for Show rules. See [the Tag::Link](enum.Tag.html#variant.Link) for
/// more information.
// TODO: support different dests.
#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ShowType {
    ShowSet,
    Function,
}

/// Type specifier for inline links. See [the Tag::Link](enum.Tag.html#variant.Link) for
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

/// Take an iterator of Typst events and output typst markup. Note that while each item
/// returned by the iterator is a `String`, each item can contain multiple lines.
// TODO: tests
pub struct TypstMarkup<'a, T> {
    tag_queue: VecDeque<Tag<'a>>,
    codeblock_queue: VecDeque<()>,
    iter: T,
}

impl<'a, T> TypstMarkup<'a, T>
where
    T: Iterator<Item = self::Event<'a>>,
{
    pub fn new(iter: T) -> Self {
        Self {
            tag_queue: VecDeque::new(),
            codeblock_queue: VecDeque::new(),
            iter,
        }
    }
}

impl<'a, T> Iterator for TypstMarkup<'a, T>
where
    T: Iterator<Item = self::Event<'a>>,
{
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            None => None,
            Some(Event::Start(x)) => {
                let ret = match x {
                    Tag::Paragraph => Some("#par()[".to_string()),
                    Tag::Show(ty, ref selector, ref set, ref func) => match ty {
                        ShowType::ShowSet => {
                            let (ele, k, v) = set.as_ref().expect("set data for show-set");
                            Some(
                                format!("#show {}: set {}({}:{})", selector, ele, k, v).to_string(),
                            )
                        }
                        ShowType::Function => Some(
                            format!(
                                "#show {}:{}",
                                selector,
                                func.as_ref().expect("function body"),
                            )
                            .to_string(),
                        ),
                    },
                    Tag::Heading(n, _, _) => Some(format!("{} ", "=".repeat(n.get().into()))),
                    // TODO: get the number of backticks / tildes somehow.
                    Tag::CodeBlock(ref fence, ref _display) => {
                        let depth = self.codeblock_queue.len();
                        self.codeblock_queue.push_back(());
                        Some(format!(
                            "{}{}\n",
                            "`".repeat(6 + depth),
                            fence
                                .clone()
                                .map(|x| x.into_string())
                                .unwrap_or_else(|| "".to_string())
                        ))
                    }
                    Tag::BulletList(_, _) => None,
                    Tag::NumberedList(_, _, _) => None,
                    Tag::Item => {
                        let list = self.tag_queue.back().expect("list item contained in list");

                        match list {
                            Tag::BulletList(_, _) => Some("- ".to_string()),
                            Tag::NumberedList(_, _, _) => Some("+ ".to_string()),
                            _ => unreachable!(),
                        }
                    }
                    Tag::Emphasis => Some("_".to_string()),
                    Tag::Strong => Some("*".to_string()),
                    Tag::Link(ref ty, ref url) => match ty {
                        LinkType::Content => Some(format!("#link(\"{url}\")[")),
                        LinkType::Url | LinkType::Autolink => Some(format!("#link(\"{url}\")")),
                    },
                    _ => todo!(),
                };

                // Set the current tag for later processing and return optional event.
                self.tag_queue.push_back(x);
                if ret.is_none() {
                    return Some("".to_string());
                }
                ret
            }
            Some(Event::End(x)) => {
                let ret = match x {
                    Tag::Paragraph => Some("]\n".to_string()),
                    Tag::Heading(_, _, _) => Some("\n".to_string()),
                    Tag::Item => Some("\n".to_string()),
                    Tag::Emphasis => Some("_".to_string()),
                    Tag::Strong => Some("*".to_string()),
                    Tag::BulletList(_, _) => Some("".to_string()),
                    Tag::NumberedList(_, _, _) => Some("".to_string()),
                    Tag::CodeBlock(_, _) => {
                        let _ = self.codeblock_queue.pop_back();
                        let depth = self.codeblock_queue.len();
                        Some(format!("{}\n", "`".repeat(6 + depth)))
                    }
                    Tag::Link(ty, _) => match ty {
                        LinkType::Content => Some("]".to_string()),
                        LinkType::Url | LinkType::Autolink => Some("".to_string()),
                    },
                    Tag::Show(_, _, _, _) => Some("\n".to_string()),
                    _ => todo!(),
                };

                let in_tag = self.tag_queue.pop_back();

                // Make sure we are in a good state.
                assert_eq!(in_tag, Some(x));
                ret
            }
            Some(Event::Raw(x)) => Some(x.into_string()),
            Some(Event::Text(x)) => Some(x.into_string()),
            Some(Event::Code(x)) => {
                // Handle a case in markdown that is invalid in typst: "````"
                if x.contains('`') {
                    Some(format!("#raw(\"{}\")", x.replace('"', r#"\""#)))
                } else {
                    Some(format!("`{x}`"))
                }
            }
            Some(Event::Linebreak) => Some("#linebreak()\n".to_string()),
            Some(Event::Parbreak) => Some("#parbreak()\n".to_string()),
            Some(Event::PageBreak) => Some("#pagebreak()\n".to_string()),
            Some(Event::Line) => todo!(),
            Some(Event::Let(lhs, rhs)) => Some(format!("#let {lhs} = {rhs}\n")),
            Some(Event::FunctionCall(v, f, args)) => {
                let args = args.join(", ");
                if let Some(v) = v {
                    Some(format!("#{v}.{f}({args})\n"))
                } else {
                    Some(format!("#{f}({args})\n"))
                }
            }
            Some(Event::Set(ele, k, v)) => Some(format!("#set {ele}({k}: {v})\n")),
        }
    }
}

/// Iterate over an Iterator of Typst [`Event`]s, generate Typst markup for each
/// [`Event`], and push it to a `String``.
pub fn push_markup<'a, T>(s: &mut String, iter: T)
where
    T: Iterator<Item = Event<'a>>,
{
    *s = TypstMarkup::new(iter).collect();
}

/// Iterate over an Iterator of Typst [`Event`]s, generate Typst markup for each
/// [`Event`], and write it to a Writer.
pub fn write_markup<'a, T, W>(w: &mut W, iter: T) -> std::io::Result<()>
where
    T: Iterator<Item = Event<'a>>,
    W: Write,
{
    for e in TypstMarkup::new(iter) {
        w.write_str(&e)
            .map_err(|e| std::io::Error::new(ErrorKind::Other, e))?;
    }
    Ok(())
}
