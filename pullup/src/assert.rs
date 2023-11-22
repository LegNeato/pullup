//! Iterator adaptors that convert markup-specific iterators into iterators that emit
//! [`ParserEvent`](crate::ParserEvent).

#[cfg(feature = "markdown")]
pub use crate::markdown::AssertMarkdown;
#[cfg(feature = "mdbook")]
pub use crate::mdbook::AssertMdbook;
#[cfg(feature = "typst")]
pub use crate::typst::AssertTypst;
