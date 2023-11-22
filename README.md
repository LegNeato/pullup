# Pullup

_Pullup_ converts between markup formats via [pulldown
parsers](https://github.com/raphlinus/pulldown-cmark#why-a-pull-parser).

Currently supported markup formats:

- [Markdown](https://commonmark.org/) (via the `markdown` feature)
- [mdBook](https://github.com/rust-lang/mdBook) (via the `mdbook` feature)
- [Typst](https://github.com/typst/typst) (via the `typst` feature)

All formats are disabled by default and must be enabled via features before use.

## How to use the crate

1. Parse markup with a format-specific pulldown parser (for example,
   [`pulldown_cmark`](https://github.com/raphlinus/pulldown-cmark) is used to parse
   Markdown). The parser creates an iterator of markup-specific `Event`s.
2. Load the format-specific `Event`s into the multi-format `ParserEvent` provided by
   this crate.
   - Iterator adaptors that do so are available in the `assert` module.
3. Operate on the `ParserEvent`s.
4. Strip irrelevant `ParserEvents` and output to a different format.
