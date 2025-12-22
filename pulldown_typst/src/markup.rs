use crate::{Event, LinkType, QuoteQuotes, QuoteType, ShowType, TableCellAlignment, Tag};
use std::{collections::VecDeque, fmt::Write, io::ErrorKind};

fn typst_escape(s: &str) -> String {
    s.replace('$', "\\$")
        .replace('#', "\\#")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('*', "\\*")
        .replace('_', " \\_")
        .replace('`', "\\`")
        .replace('@', "\\@")
}

/// Convert Typst events to Typst markup.
///
/// Note: while each item returned by the iterator is a `String`, items may contain
/// multiple lines.
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
        let event = self.iter.next();
        #[cfg(feature = "tracing")]
        tracing::trace!("TypstMarkup processing: {:?}, queue: {:?}", event, self.tag_queue);
        match event {
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
                    Tag::Heading(n, _, _, _) => Some(format!("{} ", "=".repeat(n.get().into()))),
                    // Use 6 backticks to avoid issues with code that contains triple backticks
                    // (e.g., Rust doc comments with code examples).
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
                    Tag::Emphasis => Some("#emph[".to_string()),
                    Tag::Strong => Some("#strong[".to_string()),
                    Tag::Strikethrough => Some("#strike[".to_string()),
                    Tag::Link(ref ty, ref url) => match ty {
                        LinkType::Content => {
                            // Check if url is a label reference (starts with <)
                            if url.starts_with('<') && url.ends_with('>') {
                                Some(format!("#link({url})["))
                            } else {
                                Some(format!("#link(\"{url}\")["))
                            }
                        }
                        LinkType::Url | LinkType::Autolink => Some(format!("#link(\"{url}\")[")),
                    },
                    Tag::Image(ref src, ref alt) => {
                        if alt.is_empty() {
                            Some(format!("#image(\"{src}\")"))
                        } else {
                            Some(format!("#figure(image(\"{src}\"), caption: [{alt}])"))
                        }
                    }
                    Tag::Quote(ref ty, ref quotes, ref attribution) => {
                        let block = match ty {
                            &QuoteType::Block => "block: true,",
                            &QuoteType::Inline => "block: false,",
                        };
                        let quotes = match quotes {
                            &QuoteQuotes::DoNotWrapInDoubleQuotes => "quotes: false,",
                            &QuoteQuotes::WrapInDoubleQuotes => "quotes: true,",
                            &QuoteQuotes::Auto => "quotes: auto,",
                        };
                        match attribution {
                            Some(attribution) => Some(format!(
                                "#quote({} {} attribution: [{}])[",
                                block, quotes, attribution
                            )),
                            None => Some(format!("#quote({} {})[", block, quotes)),
                        }
                    }
                    Tag::Table(ref alignment) => {
                        let columns = alignment.len();
                        let alignments = alignment
                            .iter()
                            .map(|a| match a {
                                TableCellAlignment::Left => "left",
                                TableCellAlignment::Center => "center",
                                TableCellAlignment::Right => "right",
                                TableCellAlignment::None => "auto",
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        Some(format!(
                            "#table(\n  columns: {},\n  align: ({}),\n",
                            columns, alignments
                        ))
                    }
                    Tag::TableRow => Some("  ".to_string()),
                    Tag::TableHead => Some("  table.header(".to_string()),
                    Tag::TableCell => Some("[".to_string()),
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
                    Tag::Heading(_, _, _, ref label) => Some(match label {
                        Some(l) => format!(" <{}>\n", l),
                        None => "\n".to_string(),
                    }),
                    Tag::Item => Some("\n".to_string()),
                    Tag::Emphasis => Some("]".to_string()),
                    Tag::Strong => Some("]".to_string()),
                    Tag::Strikethrough => Some("]".to_string()),
                    Tag::BulletList(_, _) => Some("".to_string()),
                    Tag::NumberedList(_, _, _) => Some("".to_string()),
                    Tag::CodeBlock(_, _) => {
                        let _ = self.codeblock_queue.pop_back();
                        let depth = self.codeblock_queue.len();
                        Some(format!("{}\n", "`".repeat(6 + depth)))
                    }
                    Tag::Link(ty, _) => match ty {
                        LinkType::Content => Some("]".to_string()),
                        LinkType::Url | LinkType::Autolink => Some("]".to_string()),
                    },
                    Tag::Image(_, _) => Some("\n".to_string()),
                    Tag::Show(_, _, _, _) => Some("\n".to_string()),
                    Tag::Quote(quote_type, _, _) => Some(match quote_type {
                        QuoteType::Inline => "]".to_string(),
                        QuoteType::Block => "]\n".to_string(),
                    }),
                    Tag::Table(_) => Some(")\n".to_string()),
                    Tag::TableHead => Some("),\n".to_string()),
                    Tag::TableRow => Some("\n".to_string()),
                    Tag::TableCell => Some("], ".to_string()),
                };

                let in_tag = self.tag_queue.pop_back();

                // Make sure we are in a good state.
                // Note: In pulldown-cmark 0.13, some End events may not carry all data from Start events.
                // For tags where this happens, we compare just the tag type rather than full equality.
                if let Some(ref start_tag) = in_tag {
                    let matches = match (&x, start_tag) {
                        // Table End events have empty alignment in pulldown-cmark 0.13
                        (Tag::Table(_), Tag::Table(_)) => true,
                        // Link End events have empty URL in pulldown-cmark 0.13
                        (Tag::Link(_, _), Tag::Link(_, _)) => true,
                        // Image End events have empty src/alt in pulldown-cmark 0.13
                        (Tag::Image(_, _), Tag::Image(_, _)) => true,
                        // CodeBlock End events may have None for language
                        (Tag::CodeBlock(_, _), Tag::CodeBlock(_, _)) => true,
                        // NumberedList End events may have default start in pulldown-cmark 0.13
                        (Tag::NumberedList(_, _, _), Tag::NumberedList(_, _, _)) => true,
                        // BulletList End events may differ
                        (Tag::BulletList(_, _), Tag::BulletList(_, _)) => true,
                        // Heading End events don't have label (only Start has it)
                        (Tag::Heading(_, _, _, _), Tag::Heading(_, _, _, _)) => true,
                        // For all other tags, require exact match
                        _ => &x == start_tag,
                    };
                    assert!(matches, "Tag mismatch: expected {:?}, got {:?}", in_tag, x);
                } else {
                    panic!("No matching start tag for end tag: {:?}", x);
                }
                ret
            }
            Some(Event::Raw(x)) => Some(x.into_string()),
            Some(Event::Text(x)) => {
                if self.codeblock_queue.is_empty() {
                    Some(typst_escape(&x))
                } else {
                    Some(x.into_string())
                }
            }
            Some(Event::Code(x)) => Some(format!(
                "#raw(\"{}\")",
                x
                    // "Raw" still needs forward slashes escaped or they will break out of
                    // the tag.
                    .replace('\\', r#"\\"#)
                    // "Raw" still needs quotes escaped or they will prematurely end the tag.
                    .replace('"', r#"\""#)
            )),
            Some(Event::Linebreak) => Some("#linebreak()\n".to_string()),
            Some(Event::Parbreak) => Some("#parbreak()\n".to_string()),
            Some(Event::PageBreak) => Some("#pagebreak()\n".to_string()),
            Some(Event::Line(start, end, length, angle, stroke)) => {
                let mut parts = vec![];

                if let Some(start) = start {
                    parts.push(format!("start: ({}, {})", start.0, start.1));
                }
                if let Some(end) = end {
                    parts.push(format!("end: ({}, {})", end.0, end.1));
                }
                if let Some(length) = length {
                    parts.push(format!("length: {}", length));
                }
                if let Some(angle) = angle {
                    parts.push(format!("angle: {}", angle));
                }
                if let Some(stroke) = stroke {
                    parts.push(format!("stroke: {}", stroke));
                }

                Some(format!("#line({})\n", parts.join(", ")))
            }
            Some(Event::Let(lhs, rhs)) => Some(format!("#let {lhs} = {rhs}\n")),
            Some(Event::FunctionCall(v, f, args)) => {
                let args = args.join(", ");
                if let Some(v) = v {
                    Some(format!("#{v}.{f}({args})\n"))
                } else {
                    Some(format!("#{f}({args})\n"))
                }
            }
            Some(Event::DocumentFunctionCall(args)) => {
                let args = args.join(", ");
                Some(format!("#document({args})\n"))
            }
            Some(Event::Set(ele, k, v)) => Some(format!("#set {ele}({k}: {v})\n")),
            Some(Event::DocumentSet(k, v)) => Some(format!("#set document({k}: {v})\n")),
            // Labels must be attached to content in Typst, so we use #[] (empty content)
            Some(Event::Label(label)) => Some(format!("#[] <{}>", label)),
        }
    }
}

/// Iterate over an Iterator of Typst [`Event`]s, generate Typst markup for each
/// [`Event`], and push it to a `String`.
pub fn push_markup<'a, T>(s: &mut String, iter: T)
where
    T: Iterator<Item = Event<'a>>,
{
    *s = TypstMarkup::new(iter).collect();
}

/// Iterate over an Iterator of Typst [`Event`]s, generate Typst markup for each
/// [`Event`], and write it to a `Write`r.
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

#[cfg(test)]
mod tests {
    use super::*;

    mod emphasis {
        use super::*;

        #[test]
        fn inline() {
            let input = vec![
                Event::Start(Tag::Emphasis),
                Event::Text("foo bar baz".into()),
                Event::End(Tag::Emphasis),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#emph[foo bar baz]";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn containing_underscores() {
            let input = vec![
                Event::Start(Tag::Emphasis),
                Event::Text("_whatever_".into()),
                Event::End(Tag::Emphasis),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#emph[ \\_whatever \\_]";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn nested() {
            let input = vec![
                Event::Start(Tag::Emphasis),
                Event::Start(Tag::Strong),
                Event::Text("blah".into()),
                Event::End(Tag::Strong),
                Event::End(Tag::Emphasis),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#emph[#strong[blah]]";
            assert_eq!(&output, &expected);
        }
    }

    mod escape {
        use super::*;

        #[test]
        fn raw_encodes_code() {
            let input = vec![Event::Code("*foo*".into())];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#raw(\"*foo*\")";
            assert_eq!(&output, &expected);
        }

        #[test]
        // https://github.com/LegNeato/mdbook-typst/issues/3
        fn raw_escapes_forward_slash() {
            let input = vec![Event::Code(r#"\"#.into())];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = r####"#raw("\\")"####;
            assert_eq!(&output, &expected);

            let input = vec![
                Event::Start(Tag::Paragraph),
                Event::Text("before ".into()),
                Event::Code(r#"\"#.into()),
                Event::Text(" after".into()),
                Event::End(Tag::Paragraph),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = r####"#par()[before #raw("\\") after]"####.to_string() + "\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn doesnt_escape_codeblock() {
            let input = vec![
                Event::Start(Tag::CodeBlock(None, crate::CodeBlockDisplay::Block)),
                Event::Text("*blah*".into()),
                Event::End(Tag::CodeBlock(None, crate::CodeBlockDisplay::Block)),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "``````\n*blah*``````\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn escapes_link_content() {
            let input = vec![
                Event::Start(Tag::Link(LinkType::Content, "http://example.com".into())),
                Event::Text("*blah*".into()),
                Event::End(Tag::Link(LinkType::Content, "http://example.com".into())),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#link(\"http://example.com\")[\\*blah\\*]";
            assert_eq!(&output, &expected);
        }
    }

    mod quote {
        use super::*;

        #[test]
        fn single() {
            let input = vec![
                Event::Start(Tag::Quote(QuoteType::Block, QuoteQuotes::Auto, None)),
                Event::Text("to be or not to be".into()),
                Event::End(Tag::Quote(QuoteType::Block, QuoteQuotes::Auto, None)),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#quote(block: true, quotes: auto,)[to be or not to be]\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn attribution() {
            let input = vec![
                Event::Start(Tag::Quote(
                    QuoteType::Block,
                    QuoteQuotes::Auto,
                    Some("some dude".into()),
                )),
                Event::Text("to be or not to be".into()),
                Event::End(Tag::Quote(
                    QuoteType::Block,
                    QuoteQuotes::Auto,
                    Some("some dude".into()),
                )),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected =
                "#quote(block: true, quotes: auto, attribution: [some dude])[to be or not to be]\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn inline_no_newline() {
            let input = vec![
                Event::Start(Tag::Quote(
                    QuoteType::Inline,
                    QuoteQuotes::Auto,
                    Some("some dude".into()),
                )),
                Event::Text("whatever".into()),
                Event::End(Tag::Quote(
                    QuoteType::Inline,
                    QuoteQuotes::Auto,
                    Some("some dude".into()),
                )),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            assert!(!output.contains('\n'));
        }

        #[test]
        fn block_has_newline() {
            let input = vec![
                Event::Start(Tag::Quote(
                    QuoteType::Block,
                    QuoteQuotes::Auto,
                    Some("some dude".into()),
                )),
                Event::Text("whatever".into()),
                Event::End(Tag::Quote(
                    QuoteType::Block,
                    QuoteQuotes::Auto,
                    Some("some dude".into()),
                )),
            ];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            assert!(output.contains('\n'));
        }
    }

    mod line {
        use super::*;

        #[test]
        fn basic() {
            let input = vec![Event::Line(None, None, None, None, None)];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#line()\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn start() {
            let input = vec![Event::Line(
                Some(("1".into(), "2".into())),
                None,
                None,
                None,
                None,
            )];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#line(start: (1, 2))\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn end() {
            let input = vec![Event::Line(
                None,
                Some(("3".into(), "4".into())),
                None,
                None,
                None,
            )];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#line(end: (3, 4))\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn length() {
            let input = vec![Event::Line(None, None, Some("5".into()), None, None)];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#line(length: 5)\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn angle() {
            let input = vec![Event::Line(None, None, None, Some("6".into()), None)];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#line(angle: 6)\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn stroke() {
            let input = vec![Event::Line(None, None, None, None, Some("7".into()))];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#line(stroke: 7)\n";
            assert_eq!(&output, &expected);
        }

        #[test]
        fn all() {
            let input = vec![Event::Line(
                Some(("1".into(), "2".into())),
                Some(("3".into(), "4".into())),
                Some("5".into()),
                Some("6".into()),
                Some("7".into()),
            )];
            let output = TypstMarkup::new(input.into_iter()).collect::<String>();
            let expected = "#line(start: (1, 2), end: (3, 4), length: 5, angle: 6, stroke: 7)\n";
            assert_eq!(&output, &expected);
        }
    }

    #[test]
    fn label_attached_to_empty_content() {
        // Labels in Typst must be attached to content, so we use #[] (empty content)
        let input = vec![Event::Label("my-anchor".into())];
        let output = TypstMarkup::new(input.into_iter()).collect::<String>();
        let expected = "#[] <my-anchor>";
        assert_eq!(&output, &expected);
    }

    #[test]
    fn table_conversion() {
        let input = vec![
            Event::Start(Tag::Table(vec![
                TableCellAlignment::Left,
                TableCellAlignment::Center,
            ])),
            Event::Start(Tag::TableHead),
            Event::Start(Tag::TableCell),
            Event::Text("Header 1".into()),
            Event::End(Tag::TableCell),
            Event::Start(Tag::TableCell),
            Event::Text("Header 2".into()),
            Event::End(Tag::TableCell),
            Event::End(Tag::TableHead),
            Event::Start(Tag::TableRow),
            Event::Start(Tag::TableCell),
            Event::Text("Cell 1".into()),
            Event::End(Tag::TableCell),
            Event::Start(Tag::TableCell),
            Event::Text("Cell 2".into()),
            Event::End(Tag::TableCell),
            Event::End(Tag::TableRow),
            Event::End(Tag::Table(vec![
                TableCellAlignment::Left,
                TableCellAlignment::Center,
            ])),
        ];

        let output = TypstMarkup::new(input.into_iter()).collect::<String>();
        let expected = "#table(\n  columns: 2,\n  align: (left, center),\n  table.header([Header 1], [Header 2], ),\n  [Cell 1], [Cell 2], \n)\n";
        assert_eq!(output, expected);
    }
}
