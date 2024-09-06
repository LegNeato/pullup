//! Convert Markdown to Typst.
use std::collections::VecDeque;

use crate::converter;
use crate::markdown;
use crate::typst;
use crate::ParserEvent;

converter!(
    /// Convert Markdown paragraphs to Typst paragraphs.
    ConvertParagraphs,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Paragraph))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Paragraph)))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Paragraph))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Paragraph)))
            },
            x => x,
    }
});

/// Convert Markdown text to Typst text.
pub struct ConvertText<T> {
    code: VecDeque<()>,
    iter: T,
}

impl<'a, T> ConvertText<T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    pub fn new(iter: T) -> Self {
        ConvertText {
            code: VecDeque::new(),
            iter,
        }
    }
}

impl<'a, T> Iterator for ConvertText<T>
where
    T: Iterator<Item = ParserEvent<'a>>,
{
    type Item = ParserEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.code.pop_back(), self.iter.next()) {
            // In code, include the unescaped text.
            (Some(_), Some(ParserEvent::Markdown(markdown::Event::Text(t)))) => {
                Some(ParserEvent::Typst(typst::Event::Text(t)))
            }
            // Not in code, escape the text using typist escaping rules.
            (None, Some(ParserEvent::Markdown(markdown::Event::Text(t)))) => {
                if t.trim().starts_with("\\[") && t.trim().ends_with("\\]") {
                    // Strip out mdbook's non-standard MathJax.
                    // TODO: Translate to typst math and/or expose this as a typed
                    // markdown event.
                    self.next()
                } else {
                    Some(ParserEvent::Typst(typst::Event::Text(t)))
                }
            }
            // Track code start.
            (
                _,
                event @ Some(ParserEvent::Markdown(markdown::Event::Start(
                    markdown::Tag::CodeBlock(_),
                ))),
            ) => {
                self.code.push_back(());
                event
            }
            // Track code end.
            (
                _,
                event @ Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::CodeBlock(
                    _,
                )))),
            ) => {
                let _ = self.code.pop_back();
                event
            }
            (_, x) => x,
        }
    }
}

converter!(
    /// Convert Markdown links to Typst links.
    ConvertLinks,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Link(kind, url, _)))) => {
                match kind {
                    markdown::LinkType::Inline => Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Link(typst::LinkType::Content, url)))),
                    /*
                    markdown::LinkType::Reference => unimplemented!(),
                    markdown::LinkType::ReferenceUnknown => unimplemented!(),
                    markdown::LinkType::Collapsed => unimplemented!(),
                    markdown::LinkType::CollapsedUnknown => unimplemented!(),
                    markdown::LinkType::Shortcut => unimplemented!(),
                    markdown::LinkType::ShortcutUnknown => unimplemented!(),
                    */
                    markdown::LinkType::Autolink => {
                        Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Link(typst::LinkType::Autolink, url))))
                    },
                    markdown::LinkType::Email => {
                        let url = "mailto:".to_string() + url.as_ref();
                        Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Link(typst::LinkType::Url, url.into()))))
                    },
                    _ => this.iter.next()
                }
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Link(kind, url, _)))) => {
                match kind {
                    markdown::LinkType::Inline => Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Link(typst::LinkType::Content, url)))),
                    /*
                    markdown::LinkType::Reference => unimplemented!(),
                    markdown::LinkType::ReferenceUnknown => unimplemented!(),
                    markdown::LinkType::Collapsed => unimplemented!(),
                    markdown::LinkType::CollapsedUnknown => unimplemented!(),
                    markdown::LinkType::Shortcut => unimplemented!(),
                    markdown::LinkType::ShortcutUnknown => unimplemented!(),
                    */
                    markdown::LinkType::Autolink => {
                        Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Link(typst::LinkType::Autolink, url))))
                    },
                    markdown::LinkType::Email => {
                        let url = "mailto:".to_string() + url.as_ref();
                        Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Link(typst::LinkType::Url, url.into()))))
                    },
                    _ => this.iter.next()
                }
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown **strong** tags to Typst strong tags.
    ConvertStrong,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Strong))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Strong)))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Strong))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Strong)))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown _emphasis_ tags to Typst emphasis tags.
    ConvertEmphasis,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Emphasis))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Emphasis)))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Emphasis))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Emphasis)))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown soft breaks to Typst line breaks.
    ConvertSoftBreaks,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::SoftBreak)) => {
                Some(ParserEvent::Typst(typst::Event::Text(" ".into())))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown hard breaks to Typst paragraph breaks.
    ConvertHardBreaks,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        // TODO: not sure that this mapping is correct.
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::HardBreak)) => {
                Some(ParserEvent::Typst(typst::Event::Parbreak))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown blockquotes to Typst quotes.
    ConvertBlockQuotes,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::BlockQuote))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Quote(typst::QuoteType::Block, typst::QuoteQuotes::Auto, None))))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::BlockQuote))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Quote(typst::QuoteType::Block, typst::QuoteQuotes::Auto, None))))
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown code tags to Typst raw tags.
    ConvertCode,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            // Inline.
            Some(ParserEvent::Markdown(markdown::Event::Code(x))) => {
                Some(ParserEvent::Typst(typst::Event::Code(x)))
            },
            // Block.
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::CodeBlock(kind)))) => {
                match kind {
                    markdown::CodeBlockKind::Indented => Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::CodeBlock(None, typst::CodeBlockDisplay::Block)))),
                    markdown::CodeBlockKind::Fenced(val) => {
                        let val = if val.as_ref() == "" {
                            None
                        } else {
                            Some(val)
                        };
                        Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::CodeBlock(val, typst::CodeBlockDisplay::Block))))
                    },
                }
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::CodeBlock(kind)))) => {
                match kind {
                    markdown::CodeBlockKind::Indented => Some(ParserEvent::Typst(typst::Event::End(typst::Tag::CodeBlock(None, typst::CodeBlockDisplay::Block)))),
                    markdown::CodeBlockKind::Fenced(val) => {
                        let val = if val.as_ref() == "" {
                            None
                        } else {
                            Some(val)
                        };
                        Some(ParserEvent::Typst(typst::Event::End(typst::Tag::CodeBlock(val, typst::CodeBlockDisplay::Block))))
                    },
                }
            },
            x => x,
    }
});

converter!(
    /// Convert Markdown lists to Typst lists.
    ConvertLists,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        // TODO: Handle tight.

        // TODO: Allow changing the marker and number format.
        match this.iter.next() {
            // List start.
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::List(number)))) => {
                if let Some(start) = number {
                    // Numbered list
                    Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::NumberedList(start, None, false))))
                } else {
                    // Bullet list
                    Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::BulletList(None, false))))
                }

            },
            // List end.
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::List(number)))) => {
                if let Some(start) = number {
                    // Numbered list
                    Some(ParserEvent::Typst(typst::Event::End(typst::Tag::NumberedList(start, None, false))))
                } else {
                    // Bullet list
                    Some(ParserEvent::Typst(typst::Event::End(typst::Tag::BulletList(None, false))))
                }

            },
            // List item start.
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Item))) => {
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Item)))
            },
            // List item end.
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Item))) => {
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Item)))
            },
            x => x,
        }
   }
);

converter!(
    /// Convert Markdown headings to Typst headings.
    ConvertHeadings,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        struct TypstLevel(std::num::NonZeroU8);

        impl std::ops::Deref for TypstLevel {
            type Target = std::num::NonZeroU8;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        impl From<markdown::HeadingLevel> for TypstLevel{
            fn from(item: markdown::HeadingLevel) -> Self {
                use markdown::HeadingLevel;
                match item {
                    HeadingLevel::H1 => TypstLevel(core::num::NonZeroU8::new(1).expect("non-zero")),
                    HeadingLevel::H2 => TypstLevel(core::num::NonZeroU8::new(2).expect("non-zero")),
                    HeadingLevel::H3 => TypstLevel(core::num::NonZeroU8::new(3).expect("non-zero")),
                    HeadingLevel::H4 => TypstLevel(core::num::NonZeroU8::new(4).expect("non-zero")),
                    HeadingLevel::H5 => TypstLevel(core::num::NonZeroU8::new(5).expect("non-zero")),
                    HeadingLevel::H6 => TypstLevel(core::num::NonZeroU8::new(6).expect("non-zero")),
                }
            }
        }
        match this.iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::Heading(level, _, _)))) => {
                let level: TypstLevel = level.into();
                Some(ParserEvent::Typst(typst::Event::Start(typst::Tag::Heading(*level,
                    typst::TableOfContents::Include,
                    typst::Bookmarks::Include,
                ))))
            },
            Some(ParserEvent::Markdown(markdown::Event::End(markdown::Tag::Heading(level, _, _))))  => {
                let level: TypstLevel = level.into();
                Some(ParserEvent::Typst(typst::Event::End(typst::Tag::Heading(*level,
                    typst::TableOfContents::Include,
                    typst::Bookmarks::Include,
                ))))
            },
            x => x,
        }
   }
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::CowStr;
    use crate::markdown::{MarkdownIter, Parser};
    use similar_asserts::assert_eq;
    use std::num::NonZeroU8;

    // Set up type names so they are clearer and more succint.
    use markdown::Event as MdEvent;
    use markdown::HeadingLevel;
    use markdown::Tag as MdTag;
    use typst::Event as TypstEvent;
    use typst::Tag as TypstTag;
    use ParserEvent::*;

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#atx-headings
    /// * https://spec.commonmark.org/0.30/#setext-headings Typst docs:
    /// * https://typst.app/docs/reference/meta/heading/
    mod headings {
        use super::*;

        #[test]
        fn convert_headings() {
            let md = "\
# Greetings

## This is **rad**!
";
            let i = ConvertHeadings::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::Heading(
                        NonZeroU8::new(1).unwrap(),
                        typst::TableOfContents::Include,
                        typst::Bookmarks::Include,
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Greetings"))),
                    Typst(TypstEvent::End(TypstTag::Heading(
                        NonZeroU8::new(1).unwrap(),
                        typst::TableOfContents::Include,
                        typst::Bookmarks::Include,
                    ))),
                    Typst(TypstEvent::Start(TypstTag::Heading(
                        NonZeroU8::new(2).unwrap(),
                        typst::TableOfContents::Include,
                        typst::Bookmarks::Include,
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("This is "))),
                    Markdown(MdEvent::Start(MdTag::Strong)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("rad"))),
                    Markdown(MdEvent::End(MdTag::Strong)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("!"))),
                    Typst(TypstEvent::End(TypstTag::Heading(
                        NonZeroU8::new(2).unwrap(),
                        typst::TableOfContents::Include,
                        typst::Bookmarks::Include,
                    ))),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#link-reference-definitions
    /// * https://spec.commonmark.org/0.30/#links
    /// * https://spec.commonmark.org/0.30/#autolinks Typst docs:
    /// * https://typst.app/docs/reference/meta/link/
    mod links {
        use super::*;
        #[test]
        fn inline() {
            let md = "\
Cool [beans](https://example.com)
";
            let i = ConvertLinks::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Cool "))),
                    Typst(TypstEvent::Start(TypstTag::Link(
                        typst::LinkType::Content,
                        CowStr::Borrowed("https://example.com")
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("beans"))),
                    Typst(TypstEvent::End(TypstTag::Link(
                        typst::LinkType::Content,
                        CowStr::Borrowed("https://example.com")
                    ))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }

        #[test]
        fn auto() {
            let md = "\
Cool <https://example.com>
";
            let i = ConvertLinks::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Cool "))),
                    Typst(TypstEvent::Start(TypstTag::Link(
                        typst::LinkType::Autolink,
                        CowStr::Borrowed("https://example.com")
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("https://example.com"))),
                    Typst(TypstEvent::End(TypstTag::Link(
                        typst::LinkType::Autolink,
                        CowStr::Borrowed("https://example.com")
                    ))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }

        #[test]
        fn email() {
            let md = "\
Who are <you@example.com>
";
            let i = ConvertLinks::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Who are "))),
                    Typst(TypstEvent::Start(TypstTag::Link(
                        typst::LinkType::Url,
                        CowStr::Boxed("mailto:you@example.com".into())
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("you@example.com"))),
                    Typst(TypstEvent::End(TypstTag::Link(
                        typst::LinkType::Url,
                        CowStr::Boxed("mailto:you@example.com".into())
                    ))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#emphasis-and-strong-emphasis Typst docs:
    /// * https://typst.app/docs/reference/text/strong/
    mod strong {
        use super::*;
        #[test]
        fn convert_strong() {
            let md = "\
## **Foo**

I **love** cake!
";
            let i = ConvertStrong::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Heading(
                        HeadingLevel::H2,
                        None,
                        vec![]
                    ))),
                    Typst(TypstEvent::Start(TypstTag::Strong)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Foo"))),
                    Typst(TypstEvent::End(TypstTag::Strong)),
                    Markdown(MdEvent::End(MdTag::Heading(HeadingLevel::H2, None, vec![]))),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("I "))),
                    Typst(TypstEvent::Start(TypstTag::Strong)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("love"))),
                    Typst(TypstEvent::End(TypstTag::Strong)),
                    Markdown(MdEvent::Text(CowStr::Borrowed(" cake!"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#emphasis-and-strong-emphasis Typst docs:
    /// * https://typst.app/docs/reference/text/emph/
    mod emphasis {
        use super::*;
        #[test]
        fn convert_emphasis() {
            let md = "\
## _Foo_

I *love* cake!
";
            let i = ConvertEmphasis::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Heading(
                        HeadingLevel::H2,
                        None,
                        vec![]
                    ))),
                    Typst(TypstEvent::Start(TypstTag::Emphasis)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("Foo"))),
                    Typst(TypstEvent::End(TypstTag::Emphasis)),
                    Markdown(MdEvent::End(MdTag::Heading(HeadingLevel::H2, None, vec![]))),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("I "))),
                    Typst(TypstEvent::Start(TypstTag::Emphasis)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("love"))),
                    Typst(TypstEvent::End(TypstTag::Emphasis)),
                    Markdown(MdEvent::Text(CowStr::Borrowed(" cake!"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#code Typst docs:
    /// * https://typst.app/docs/reference/text/raw/
    mod code {
        use super::*;
        #[test]
        fn inline() {
            let md = "\
foo `bar` baz
";
            let i = ConvertCode::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("foo "))),
                    Typst(TypstEvent::Code(CowStr::Borrowed("bar"))),
                    Markdown(MdEvent::Text(CowStr::Borrowed(" baz"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }

        #[test]
        fn block_indent() {
            let md = "\
whatever

    code 1
    code 2
";
            let i = ConvertCode::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("whatever"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                    Typst(TypstEvent::Start(TypstTag::CodeBlock(
                        None,
                        typst::CodeBlockDisplay::Block
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("code 1\n"))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("code 2\n"))),
                    Typst(TypstEvent::End(TypstTag::CodeBlock(
                        None,
                        typst::CodeBlockDisplay::Block
                    ))),
                ]
            );
        }

        #[test]
        fn block() {
            let md = "\
```
blah
```
";
            let i = ConvertCode::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::CodeBlock(
                        None,
                        typst::CodeBlockDisplay::Block
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("blah\n"))),
                    Typst(TypstEvent::End(TypstTag::CodeBlock(
                        None,
                        typst::CodeBlockDisplay::Block
                    ))),
                ]
            );
        }

        #[test]
        fn block_with_fence() {
            let md = "\
```foo
blah
```
";
            let i = ConvertCode::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::CodeBlock(
                        Some(CowStr::Borrowed("foo")),
                        typst::CodeBlockDisplay::Block
                    ))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("blah\n"))),
                    Typst(TypstEvent::End(TypstTag::CodeBlock(
                        Some(CowStr::Borrowed("foo")),
                        typst::CodeBlockDisplay::Block
                    ))),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#text Typst docs:
    /// * https://typst.app/docs/reference/text/
    mod text {
        use super::*;
        #[test]
        fn convert_text() {
            let md = "\
foo

bar

baz
";
            let i = ConvertText::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Typst(TypstEvent::Text(CowStr::Borrowed("foo"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Typst(TypstEvent::Text(CowStr::Borrowed("bar"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Typst(TypstEvent::Text(CowStr::Borrowed("baz"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#text
    /// Typst docs:
    /// * https://typst.app/docs/reference/text/
    mod breaks {
        use super::*;
        #[test]
        fn soft() {
            let md = "\
foo
bar
";
            let i = ConvertSoftBreaks::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("foo"))),
                    Typst(TypstEvent::Text(CowStr::Borrowed(" "))),
                    Markdown(MdEvent::Text(CowStr::Borrowed("bar"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }

        #[test]
        fn hard() {
            // Note that "foo" has two spaces after it.
            let md = "\
foo  
bar
";
            let i = ConvertHardBreaks::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("foo"))),
                    Typst(TypstEvent::Parbreak),
                    Markdown(MdEvent::Text(CowStr::Borrowed("bar"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#paragraphs Typst docs:
    /// * https://typst.app/docs/reference/layout/par/
    mod paragraphs {
        use super::*;
        #[test]
        fn convert_paragraphs() {
            let md = "\
foo

bar

baz
";
            let i = ConvertParagraphs::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("foo"))),
                    Typst(TypstEvent::End(TypstTag::Paragraph)),
                    Typst(TypstEvent::Start(TypstTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("bar"))),
                    Typst(TypstEvent::End(TypstTag::Paragraph)),
                    Typst(TypstEvent::Start(TypstTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("baz"))),
                    Typst(TypstEvent::End(TypstTag::Paragraph)),
                ]
            );
        }
    }

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#lists Typst docs:
    /// * https://typst.app/docs/reference/layout/list
    /// * https://typst.app/docs/reference/layout/enum/
    mod lists {
        use super::*;

        #[test]
        fn bullet() {
            let md = "\
* dogs
* are
* cool
";
            let i = ConvertLists::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::BulletList(None, false))),
                    // First bulet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("dogs"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    // Second bullet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("are"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    // Third bullet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("cool"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    Typst(TypstEvent::End(TypstTag::BulletList(None, false))),
                ],
            );
        }

        #[test]
        fn numbered() {
            let md = "\
1. cats are _too_
2. birds are ok
";
            let i = ConvertLists::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::NumberedList(1, None, false))),
                    // First bullet
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("cats are "))),
                    Markdown(MdEvent::Start(MdTag::Emphasis)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("too"))),
                    Markdown(MdEvent::End(MdTag::Emphasis)),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    // Second bullet
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("birds are ok"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    Typst(TypstEvent::End(TypstTag::NumberedList(1, None, false))),
                ],
            );
        }

        #[test]
        fn numbered_custom_start() {
            let md = "\
6. foo
1. bar
";
            let i = ConvertLists::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::NumberedList(6, None, false))),
                    // First bullet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("foo"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    // Second bullet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("bar"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    Typst(TypstEvent::End(TypstTag::NumberedList(6, None, false))),
                ],
            );
        }

        #[test]
        fn multiple_lines() {
            let md = "\
* multiple
  lines
";
            let i = ConvertLists::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::BulletList(None, false))),
                    // First bullet.
                    Typst(TypstEvent::Start(TypstTag::Item)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("multiple"))),
                    Markdown(MdEvent::SoftBreak),
                    Markdown(MdEvent::Text(CowStr::Borrowed("lines"))),
                    Typst(TypstEvent::End(TypstTag::Item)),
                    Typst(TypstEvent::End(TypstTag::BulletList(None, false))),
                ]
            );
        }
    }

    mod issues {
        use super::*;

        // https://github.com/LegNeato/mdbook-typst/issues/3
        #[test]
        fn backslashes_in_backticks() {
            let md = r###"before `\` after"###;

            let i = ConvertText::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Typst(TypstEvent::Text("before ".into())),
                    Markdown(MdEvent::Code(r#"\"#.into())),
                    Typst(TypstEvent::Text(" after".into())),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                ],
            );
        }

        // https://github.com/LegNeato/mdbook-typst/issues/9
        #[test]
        fn simple_blockquote() {
            let md = "> test";

            let i = ConvertBlockQuotes::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::Quote(
                        typst::QuoteType::Block,
                        typst::QuoteQuotes::Auto,
                        None,
                    ))),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("test"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                    Typst(TypstEvent::End(TypstTag::Quote(
                        typst::QuoteType::Block,
                        typst::QuoteQuotes::Auto,
                        None,
                    ))),
                ],
            );
        }

        // https://github.com/LegNeato/mdbook-typst/issues/9
        #[test]
        fn complex_blockquote() {
            let md = "> one\n> two\n> three";

            let i = ConvertBlockQuotes::new(MarkdownIter(Parser::new(&md)));

            self::assert_eq!(
                i.collect::<Vec<super::ParserEvent>>(),
                vec![
                    Typst(TypstEvent::Start(TypstTag::Quote(
                        typst::QuoteType::Block,
                        typst::QuoteQuotes::Auto,
                        None,
                    ))),
                    Markdown(MdEvent::Start(MdTag::Paragraph)),
                    Markdown(MdEvent::Text(CowStr::Borrowed("one"))),
                    Markdown(MdEvent::SoftBreak),
                    Markdown(MdEvent::Text(CowStr::Borrowed("two"))),
                    Markdown(MdEvent::SoftBreak),
                    Markdown(MdEvent::Text(CowStr::Borrowed("three"))),
                    Markdown(MdEvent::End(MdTag::Paragraph)),
                    Typst(TypstEvent::End(TypstTag::Quote(
                        typst::QuoteType::Block,
                        typst::QuoteQuotes::Auto,
                        None,
                    ))),
                ],
            );
        }
    }
}
