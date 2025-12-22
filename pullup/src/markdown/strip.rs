use crate::converter;
use crate::markdown;
use crate::ParserEvent;

converter!(
    /// Strip out Markdown HTML.
    StripHtml,
    ParserEvent<'a> => ParserEvent<'a>,
    |this: &mut Self| {
        match this.iter.next() {
            // Strip block-level HTML (start tag)
            Some(ParserEvent::Markdown(markdown::Event::Start(markdown::Tag::HtmlBlock))) => {
                // Skip until we find the end of the HTML block
                loop {
                    match this.iter.next() {
                        Some(ParserEvent::Markdown(markdown::Event::End(markdown::TagEnd::HtmlBlock))) => break,
                        None => return None,
                        _ => continue,
                    }
                }
                this.next()
            },
            // Strip Html events (content inside HtmlBlock)
            Some(ParserEvent::Markdown(markdown::Event::Html(_))) => {
                this.iter.find(|x| !matches!(x, ParserEvent::Markdown(markdown::Event::Html(_))))
            },
            // Strip inline HTML
            Some(ParserEvent::Markdown(markdown::Event::InlineHtml(_))) => {
                this.next()
            },
            x => x,
    }
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::CowStr;
    use crate::markdown::*;
    use similar_asserts::assert_eq;

    // Set up type names so they are clearer and more succint.
    use crate::markdown::Event as MdEvent;
    use crate::markdown::Tag as MdTag;
    use crate::markdown::TagEnd as MdTagEnd;

    /// Markdown docs:
    /// * https://spec.commonmark.org/0.30/#html-blocks
    mod html {
        use super::*;
        #[test]
        fn strip_html() {
            let md = "\
# Hello

<html>
is anybody in there?
</html>

*just nod if you can hear me*
<del>*foo*</del>
";
            let i = AssertMarkdown(super::StripHtml::new(MarkdownIter(Parser::new(&md))));
            self::assert_eq!(
                i.collect::<Vec<markdown::Event>>(),
                vec![
                    MdEvent::Start(MdTag::Heading {
                        level: HeadingLevel::H1,
                        id: None,
                        classes: vec![],
                        attrs: vec![]
                    }),
                    MdEvent::Text(CowStr::Borrowed("Hello")),
                    MdEvent::End(MdTagEnd::Heading(HeadingLevel::H1)),
                    MdEvent::Start(MdTag::Paragraph),
                    MdEvent::Start(MdTag::Emphasis),
                    MdEvent::Text(CowStr::Borrowed("just nod if you can hear me")),
                    MdEvent::End(MdTagEnd::Emphasis),
                    MdEvent::SoftBreak,
                    MdEvent::Start(MdTag::Emphasis),
                    MdEvent::Text(CowStr::Borrowed("foo")),
                    MdEvent::End(MdTagEnd::Emphasis),
                    MdEvent::End(MdTagEnd::Paragraph),
                ]
            );
        }
    }
}
