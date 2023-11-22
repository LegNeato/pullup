use crate::converter;
use crate::markdown;
use crate::ParserEvent;

converter!(
    /// Strip out Markdown HTML.
    StripHtml,
    ParserEvent<'a> => ParserEvent<'a>,
    |iter: &mut I| {
        match iter.next() {
            Some(ParserEvent::Markdown(markdown::Event::Html(_))) => {
                iter.find(|x| !matches!(x, ParserEvent::Markdown(markdown::Event::Html(_))))
            },
            x => x,
    }
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::*;
    use crate::markdown::CowStr;
    use similar_asserts::assert_eq;
    use std::num::NonZeroU8;

    // Set up type names so they are clearer and more succint.
    use crate::markdown::Event as MdEvent;
    use crate::markdown::Tag as MdTag;
    use crate::typst::Event as TypstEvent;
    use crate::typst::Tag as TypstTag;

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
                    MdEvent::Start(Tag::Heading(HeadingLevel::H1, None, vec![])),
                    MdEvent::Text(CowStr::Borrowed("Hello")),
                    MdEvent::End(MdTag::Heading(HeadingLevel::H1, None, vec![])),
                    MdEvent::Start(MdTag::Paragraph),
                    MdEvent::Start(MdTag::Emphasis),
                    MdEvent::Text(CowStr::Borrowed("just nod if you can hear me")),
                    MdEvent::End(MdTag::Emphasis),
                    MdEvent::SoftBreak,
                    MdEvent::Start(MdTag::Emphasis),
                    MdEvent::Text(CowStr::Borrowed("foo")),
                    MdEvent::End(MdTag::Emphasis),
                    MdEvent::End(MdTag::Paragraph),
                ]
            );
        }
    }
}
