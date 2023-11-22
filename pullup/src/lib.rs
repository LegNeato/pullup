pub mod assert;
pub mod filter;

#[cfg(feature = "markdown")]
pub mod markdown;
#[cfg(feature = "mdbook")]
pub mod mdbook;
#[cfg(feature = "typst")]
pub mod typst;

/// Represents all the types of markup events this crate can operate on.
///
/// Markup adapters:
/// * Convert format-specific iterators to [`ParserEvent`] iterators for further
///   processing.
/// * Convert [`ParserEvent`] iterators to format-specific iterators for output.
#[derive(Debug, PartialEq)]
pub enum ParserEvent<'a> {
    #[cfg(feature = "markdown")]
    Markdown(markdown::Event<'a>),
    #[cfg(feature = "mdbook")]
    Mdbook(mdbook::Event<'a>),
    #[cfg(feature = "typst")]
    Typst(typst::Event<'a>),
    #[cfg(not(any(feature = "markdown", feature = "mdbook", feature = "typst")))]
    NoFeaturesEnabled(core::marker::PhantomData<&'a ()>),
}

#[macro_export]
/// Convert between markup events without a buffer or lookahead.
macro_rules! converter {
    (
        $(#[$attr:meta])*
        $struct_name:ident,
        $in:ty => $out:ty,
        $body:expr
    ) => {
        // Define the struct with the given name
        #[derive(Debug, Clone)]
        $(#[$attr])*
        pub struct $struct_name<'a, I> {
            iter: I,
            p: core::marker::PhantomData<&'a ()>,
        }

        // Define an implementation for the struct
        impl<'a, I> $struct_name<'a, I>
        where
            I: Iterator<Item = ParserEvent<'a>>,
        {
            #[allow(dead_code)]
            pub fn new(iter: I) -> Self {
                Self {
                    iter,
                    p: core::marker::PhantomData,
                }
            }
        }

        impl<'a, I> Iterator for $struct_name<'a, I>
        where
            I: Iterator<Item = $in>,
        {
            type Item = $out;

            fn next(&mut self) -> Option<Self::Item> {
                #[allow(clippy::redundant_closure_call)]
                $body(&mut self.iter)
            }
        }
    };
}
