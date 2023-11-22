//! Parsers to Convert mdBook into an [`Event`] iterator.
use crate::*;
use core::iter;
use mdbook::{renderer::RenderContext, BookItem, Config, MDBook};

#[derive(Default, Debug)]
enum ConfigState {
    #[default]
    Start,
    Title,
    AuthorList,
    Author(usize),
    Done,
}

/// Parse an mdBook configuration into events.
#[derive(Debug)]
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
                    .first()
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
                    let p = TextMergeStream::new(pulldown_cmark::Parser::new(&ch.content));
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

/// Parse an mdBook structure into events.
// TODO: tests
#[derive(Debug, Clone)]
pub struct Parser<'a>(Vec<self::Event<'a>>);

impl<'a> Parser<'a> {
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

impl<'a> Iterator for Parser<'a> {
    type Item = self::Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.first().cloned()
    }
}
