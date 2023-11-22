# Pullup

**Pullup** converts between [*pull*down
parser](https://github.com/raphlinus/pulldown-cmark#why-a-pull-parser) events for
various mark*up* formats.

Currently supported markup formats:

- [Markdown](https://commonmark.org/) (via the `markdown` feature)
- [mdBook](https://github.com/rust-lang/mdBook) (via the `mdbook` feature)
- [Typst](https://github.com/typst/typst) (via the `typst` feature)

Formats are disabled by default and must be enabled via features before use.
