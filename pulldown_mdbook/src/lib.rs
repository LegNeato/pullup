use core::iter;
use mdbook::{renderer::RenderContext, BookItem, Config, MDBook};
use pulldown_cmark::{CowStr, Parser};
use std::path::PathBuf;

pub mod markdown;

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

#[derive(Default, Debug)]
enum ConfigState {
    #[default]
    Start,
    Title,
    AuthorList,
    Author(usize),
    Done,
}

pub struct ConfigParser<'a> {
    state: ConfigState,
    config: &'a Config,
}

impl<'a> ConfigParser<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self {
            config,
            state: ConfigState::default(),
        }
    }
}
impl<'a> Iterator for ConfigParser<'a> {
    type Item = self::Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            ConfigState::Start => {
                self.state = ConfigState::Title;
                if let Some(title) = self.config.book.title.as_ref() {
                    Some(self::Event::Title(title.clone().into()))
                } else {
                    self.next()
                }
            }
            ConfigState::Title => {
                if !self.config.book.authors.is_empty() {
                    self.state = ConfigState::AuthorList;
                    Some(Event::Start(Tag::AuthorList))
                } else {
                    self.state = ConfigState::Done;
                    self.next()
                }
            }
            ConfigState::AuthorList => {
                self.state = ConfigState::Author(1);
                let author = self
                    .config
                    .book
                    .authors
                    .get(0)
                    .expect("author in author list");
                Some(Event::Author(author.clone().into()))
            }
            ConfigState::Author(index) => {
                if index >= self.config.book.authors.len() {
                    self.state = ConfigState::Done;
                    Some(Event::End(Tag::AuthorList))
                } else {
                    self.state = ConfigState::Author(index + 1);
                    let author = self
                        .config
                        .book
                        .authors
                        .get(index)
                        .expect("author index under length");
                    Some(Event::Author(author.clone().into()))
                }
            }
            ConfigState::Done => None,
        }
    }
}

// This is super janky. I have a branch breaking this into iterators but am stuck with
// lifetimes not living long enough for the embedded markdown events.
fn events_from_items<'a, 'b>(items: &'a [BookItem]) -> Vec<self::Event<'a>>
where
    'a: 'b,
{
    let (_, events) = items
        .iter()
        .fold((None, vec![]), |mut acc, item| match item {
            BookItem::Chapter(ch) => {
                let status = if ch.is_draft_chapter() {
                    ChapterStatus::Draft
                } else {
                    ChapterStatus::Active
                };
                let name = ch.name.clone();
                let source = ch
                    .source_path
                    .as_ref()
                    .map(|x| ChapterSource::Path(x.to_owned()));

                // Chapter start event.
                acc.1.push(self::Event::Start(self::Tag::Chapter(
                    status,
                    name.clone().into(),
                    source.clone(),
                    None,
                )));

                // Chapter content events.
                if !ch.content.is_empty() {
                    let p = TextMergeStream::new(Parser::new(&ch.content));
                    let p = p.map(Event::MarkdownContentEvent);
                    acc.1.extend(p);
                    acc.1.push(Event::End(Tag::Content(ContentType::Markdown)));
                };

                if !ch.sub_items.is_empty() {
                    let subevents = events_from_items(&ch.sub_items);
                    acc.1.extend(subevents);
                }

                // Chapter end event.
                acc.1
                    .push(Event::End(Tag::Chapter(status, name.into(), source, None)));
                acc
            }
            BookItem::Separator => {
                acc.1.push(self::Event::Separator);
                acc
            }
            // TODO: numbering.
            BookItem::PartTitle(x) => {
                let ev = if let Some(current_title) = acc.0 {
                    // Close the current part and start a new one.
                    vec![
                        self::Event::End(self::Tag::Part(Some(current_title), None)),
                        self::Event::Start(self::Tag::Part(Some(x.clone().into()), None)),
                    ]
                } else {
                    // Start a new part with a title.
                    vec![self::Event::Start(self::Tag::Part(
                        Some(x.clone().into()),
                        None,
                    ))]
                };
                acc.1.extend(ev);
                (Some(x.clone().into()), acc.1)
            }
        });

    // Post-process to insert `Part` events. The mdbook data model kinda has these,
    // kinda does not. We'll be consistent and make all chapters contained in parts.
    let first_start_pos = events
        .iter()
        .position(|x| matches!(x, self::Event::Start(self::Tag::Part(_, _))));
    let first_end_pos = events
        .iter()
        .position(|x| matches!(x, self::Event::End(self::Tag::Part(_, _))));
    let last_start_pos = events
        .iter()
        .rposition(|x| matches!(x, self::Event::Start(self::Tag::Part(_, _))));
    let last_end_pos = events
        .iter()
        .rposition(|x| matches!(x, self::Event::End(self::Tag::Part(_, _))));

    match (first_start_pos, first_end_pos, last_start_pos, last_end_pos) {
        // No parts / titles at all, wrap the whole thing in an untitled part.
        (None, None, _, _) => iter::once(self::Event::Start(self::Tag::Part(None, None)))
            .chain(events)
            .chain(iter::once(self::Event::End(self::Tag::Part(None, None))))
            .collect(),
        // Only ends, missing starts.
        (None, Some(p), None, Some(_)) => {
            // Add a start for the first end.
            let start = match &events[p] {
                Event::End(tag) => Event::Start(tag.clone()),
                _ => unreachable!(),
            };
            iter::once(start).chain(events.iter().cloned()).collect()
        }
        // Only starts, missing ends.
        (Some(first_start), None, Some(last_start), None) => {
            // Add an end for the last start.
            let end = match &events[last_start] {
                Event::Start(tag) => Event::End(tag.clone()),
                _ => unreachable!(),
            };
            if first_start != 0 {
                // Synthesize starting part, and end the first part where the second
                // part starts.
                let (inside, outside) = events.split_at(first_start);
                return iter::once(Event::Start(Tag::Part(None, None)))
                    .chain(inside.iter().cloned())
                    .chain(iter::once(Event::End(Tag::Part(None, None))))
                    .chain(outside.iter().cloned())
                    // Put our new end on.
                    .chain(iter::once(end))
                    .collect();
            } else {
                // Just the end.
                events.iter().cloned().chain(iter::once(end)).collect()
            }
        }

        // Contains both starts and ends, we need to make sure they are matched.
        (Some(first_start), Some(first_end), Some(last_start), Some(last_end)) => {
            // End before start, so we need to create a start for the first end.
            if first_end < first_start {
                let start = match &events[first_end] {
                    Event::End(tag) => Event::Start(tag.clone()),
                    _ => unreachable!(),
                };
                return iter::once(start).chain(events.iter().cloned()).collect();
            }

            // Start after end, so we need to create an end for the last start.
            if last_start > last_end {
                let end = match &events[last_start] {
                    Event::Start(tag) => Event::End(tag.clone()),
                    _ => unreachable!(),
                };
                return events.iter().cloned().chain(iter::once(end)).collect();
            }

            // Everything is matched, just return what is there.
            events
        }
        // If we find a part, it will be found forwards and backwards. Therefore, the
        // other combinations are not possible.
        _ => unreachable!(),
    }
}

/// Parse an MdBook structure into events.
// TODO: tests
#[derive(Debug, Clone)]
pub struct MdBookParser<'a>(Vec<self::Event<'a>>);

impl<'a> MdBookParser<'a> {
    /// Create a parser from an `MDBook`. This is available when using `mdbook` as a
    /// library.
    pub fn from_mdbook(book: &'a MDBook) -> Self {
        let config = ConfigParser::new(&book.config);
        let events = vec![self::Event::Root(book.root.to_owned().clone())]
            .into_iter()
            .chain(config)
            .chain(events_from_items(&book.book.sections))
            .collect();
        Self(events)
    }

    /// Create a parser from a `RenderContext`. This is available when using `mdbook` as
    /// a binary.
    pub fn from_rendercontext(ctx: &'a RenderContext) -> Self {
        let config = ConfigParser::new(&ctx.config);
        let events = vec![self::Event::Root(ctx.root.clone())]
            .into_iter()
            .chain(config)
            .chain(events_from_items(&ctx.book.sections))
            .collect();

        Self(events)
    }

    pub fn iter(&self) -> impl Iterator<Item = &self::Event<'a>> {
        self.0.iter()
    }
}

impl<'a> Iterator for MdBookParser<'a> {
    type Item = self::Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.first().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CowStr;
    use similar_asserts::assert_eq;

    // Set up type names so they are clearer.
    use Event as MdbookEvent;
    use Tag as MdbookTag;

    mod parts {
        use mdbook::book::Chapter;

        use super::*;

        fn ch1() -> Chapter {
            Chapter {
                name: "Chapter 1".to_string(),
                ..Default::default()
            }
        }
        fn ch2() -> Chapter {
            Chapter {
                name: "Chapter 2".to_string(),
                ..Default::default()
            }
        }

        fn inlined<'a>(x: &'a str) -> CowStr<'a> {
            CowStr::Inlined(x.try_into().unwrap())
        }
        fn boxed<'a>(x: &'a str) -> CowStr<'a> {
            CowStr::Boxed(x.try_into().unwrap())
        }

        #[test]
        fn no_parts() {
            let input = vec![mdbook::BookItem::Chapter(ch1())];
            let output = events_from_items(&input);
            self::assert_eq!(
                output,
                vec![
                    MdbookEvent::Start(MdbookTag::Part(None, None)),
                    MdbookEvent::Start(MdbookTag::Chapter(
                        ChapterStatus::Draft,
                        ch1().name.into(),
                        None,
                        None
                    )),
                    MdbookEvent::End(MdbookTag::Chapter(
                        ChapterStatus::Draft,
                        ch1().name.into(),
                        None,
                        None
                    )),
                    MdbookEvent::End(MdbookTag::Part(None, None)),
                ],
            );
        }

        #[test]
        fn some_parts() {
            let input = vec![
                mdbook::BookItem::Chapter(ch1()),
                mdbook::BookItem::PartTitle("between".to_string()),
                mdbook::BookItem::Chapter(ch2()),
            ];
            let output = events_from_items(&input);
            self::assert_eq!(
                output,
                vec![
                    // Synthesized untitled part with chapter 1.
                    MdbookEvent::Start(MdbookTag::Part(None, None)),
                    MdbookEvent::Start(MdbookTag::Chapter(
                        ChapterStatus::Draft,
                        inlined(&ch1().name),
                        None,
                        None
                    )),
                    MdbookEvent::End(MdbookTag::Chapter(
                        ChapterStatus::Draft,
                        inlined(&ch1().name),
                        None,
                        None
                    )),
                    MdbookEvent::End(MdbookTag::Part(None, None)),
                    // Second part with title and chapter 2.
                    MdbookEvent::Start(MdbookTag::Part(Some(inlined("between")), None)),
                    MdbookEvent::Start(MdbookTag::Chapter(
                        ChapterStatus::Draft,
                        inlined(&ch2().name),
                        None,
                        None
                    )),
                    MdbookEvent::End(MdbookTag::Chapter(
                        ChapterStatus::Draft,
                        inlined(&ch2().name),
                        None,
                        None
                    )),
                    MdbookEvent::End(MdbookTag::Part(Some(inlined("between")), None)),
                ],
            );
        }

        #[test]
        fn all_parts() {
            let input = vec![
                mdbook::BookItem::PartTitle("one".to_string()),
                mdbook::BookItem::Chapter(ch1()),
                mdbook::BookItem::PartTitle("twox".to_string()),
                mdbook::BookItem::Chapter(ch2()),
            ];
            let output = events_from_items(&input);
            std::assert_eq!(
                output,
                vec![
                    // Part 1.
                    MdbookEvent::Start(MdbookTag::Part(Some(boxed("one")), None)),
                    MdbookEvent::Start(MdbookTag::Chapter(
                        ChapterStatus::Draft,
                        ch1().name.into(),
                        None,
                        None
                    )),
                    MdbookEvent::End(MdbookTag::Chapter(
                        ChapterStatus::Draft,
                        ch1().name.into(),
                        None,
                        None
                    )),
                    MdbookEvent::End(MdbookTag::Part(Some(boxed("one")), None)),
                    // Part 2.
                    MdbookEvent::Start(MdbookTag::Part(Some(boxed("twox")), None)),
                    MdbookEvent::Start(MdbookTag::Chapter(
                        ChapterStatus::Draft,
                        ch2().name.into(),
                        None,
                        None
                    )),
                    MdbookEvent::End(MdbookTag::Chapter(
                        ChapterStatus::Draft,
                        ch2().name.into(),
                        None,
                        None
                    )),
                    MdbookEvent::End(MdbookTag::Part(Some(boxed("twox")), None)),
                ],
            );
        }
    }
}
