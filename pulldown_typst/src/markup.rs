use crate::{Event, LinkType, ShowType, Tag};
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
                    Tag::Emphasis => Some("#emph[".to_string()),
                    Tag::Strong => Some("#strong[".to_string()),
                    Tag::Link(ref ty, ref url) => match ty {
                        LinkType::Content => Some(format!("#link(\"{url}\")[")),
                        LinkType::Url | LinkType::Autolink => Some(format!("#link(\"{url}\")[")),
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
                    Tag::Emphasis => Some("]".to_string()),
                    Tag::Strong => Some("]".to_string()),
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
                    Tag::Show(_, _, _, _) => Some("\n".to_string()),
                    _ => todo!(),
                };

                let in_tag = self.tag_queue.pop_back();

                // Make sure we are in a good state.
                assert_eq!(in_tag, Some(x));
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
            Some(Event::Code(x)) => Some(format!("#raw(\"{}\")", x.replace('"', r#"\""#))),
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
            Some(Event::DocumentFunctionCall(args)) => {
                let args = args.join(", ");
                Some(format!("#document({args})\n"))
            }
            Some(Event::Set(ele, k, v)) => Some(format!("#set {ele}({k}: {v})\n")),
            Some(Event::DocumentSet(k, v)) => Some(format!("#set document({k}: {v})\n")),
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
}
