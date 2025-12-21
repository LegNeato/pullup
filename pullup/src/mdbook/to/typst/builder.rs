//! Builder to customize mdBook to Typst conversion.

use core::marker::PhantomData;

use crate::markdown::to::typst::*;
use crate::mdbook::to::typst::*;
use crate::mdbook::MdbookIter;
use crate::ParserEvent;

#[derive(typed_builder::TypedBuilder)]
#[builder(build_method(vis="", name=__build))]
#[builder(field_defaults(default = true))]
/// Converts Mdbook to Typst.
///
/// Using the builder one can choose which conversions to apply. By default, all
/// conversions are enabled.
///
/// For more control over conversion, use the converters in [the parent
/// module](crate::mdbook::to::typst) and [markdown module](crate::markdown::to::typst)
/// directly. Additionally, one may turn off the conversion via the builder and operate
/// on the resulting [`ParseEvent`](crate::ParserEvent) iterator.
pub struct Conversion<'a, T> {
    #[builder(!default)]
    events: T,
    title: bool,
    authors: bool,
    chapters: bool,
    content: bool,
    headings: bool,
    paragraphs: bool,
    soft_breaks: bool,
    hard_breaks: bool,
    text: bool,
    strong: bool,
    emphasis: bool,
    blockquotes: bool,
    lists: bool,
    code: bool,
    links: bool,
    images: bool,
    tables: bool,
    #[builder(default)]
    _p: PhantomData<&'a ()>,
}

#[allow(dead_code, non_camel_case_types, missing_docs)]
#[automatically_derived]
impl<
        'a,
        T,
        __title: ::typed_builder::Optional<bool>,
        __authors: ::typed_builder::Optional<bool>,
        __chapters: ::typed_builder::Optional<bool>,
        __content: ::typed_builder::Optional<bool>,
        __headings: ::typed_builder::Optional<bool>,
        __paragraphs: ::typed_builder::Optional<bool>,
        __soft_breaks: ::typed_builder::Optional<bool>,
        __hard_breaks: ::typed_builder::Optional<bool>,
        __text: ::typed_builder::Optional<bool>,
        __strong: ::typed_builder::Optional<bool>,
        __emphasis: ::typed_builder::Optional<bool>,
        __blockquotes: ::typed_builder::Optional<bool>,
        __lists: ::typed_builder::Optional<bool>,
        __code: ::typed_builder::Optional<bool>,
        __links: ::typed_builder::Optional<bool>,
        __images: ::typed_builder::Optional<bool>,
        __tables: ::typed_builder::Optional<bool>,
        ___p: ::typed_builder::Optional<PhantomData<&'a ()>>,
    >
    ConversionBuilder<
        'a,
        T,
        (
            (T,),
            __title,
            __authors,
            __chapters,
            __content,
            __headings,
            __paragraphs,
            __soft_breaks,
            __hard_breaks,
            __text,
            __strong,
            __emphasis,
            __blockquotes,
            __lists,
            __code,
            __links,
            __images,
            __tables,
            ___p,
        ),
    >
where
    T: Iterator<Item = crate::mdbook::Event<'a>> + 'a,
{
    pub fn build(self) -> impl Iterator<Item = ParserEvent<'a>> {
        let this = self.__build();
        let mut events: Box<dyn Iterator<Item = ParserEvent<'_>>> =
            Box::new(MdbookIter(this.events));
        if this.title {
            events = Box::new(ConvertTitle::new(events));
        }
        if this.authors {
            events = Box::new(ConvertAuthors::new(events));
        }
        if this.chapters {
            events = Box::new(ConvertChapter::new(events));
        }
        if this.content {
            events = Box::new(events.map(|e| match e {
                ParserEvent::Mdbook(mdbook::Event::MarkdownContentEvent(m)) => {
                    ParserEvent::Markdown(m)
                }
                x => x,
            }));
            if this.headings {
                events = Box::new(ConvertHeadings::new(events));
            }
            if this.paragraphs {
                events = Box::new(ConvertParagraphs::new(events));
            }
            if this.soft_breaks {
                events = Box::new(ConvertSoftBreaks::new(events));
            }
            if this.hard_breaks {
                events = Box::new(ConvertHardBreaks::new(events));
            }
            if this.text {
                events = Box::new(ConvertText::new(events));
            }
            if this.strong {
                events = Box::new(ConvertStrong::new(events));
            }
            if this.emphasis {
                events = Box::new(ConvertEmphasis::new(events));
            }
            if this.blockquotes {
                events = Box::new(ConvertBlockQuotes::new(events));
            }
            if this.lists {
                events = Box::new(ConvertLists::new(events));
            }
            if this.code {
                events = Box::new(ConvertCode::new(events));
            }
            if this.links {
                events = Box::new(ConvertLinks::new(events));
            }
            if this.images {
                events = Box::new(ConvertImages::new(events));
            }
            if this.tables {
                events = Box::new(ConvertTables::new(events));
            }
        }

        events
    }
}
