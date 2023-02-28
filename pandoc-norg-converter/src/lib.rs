//! A library to convert from [neorg] to [pandoc].
//!
//! Start by taking a look at the [`Frontend`] documentation.
//!
//! # Example
//!
//! ```rust
//! use pandoc_norg_converter::Frontend;
//!
//! let norg_source = r#"
//! ## My amazing document
//!
//! This is my amazing document built with *Neorg*
//! "#;
//!
//! let mut frontend = Frontend::default();
//! let document = frontend.convert(norg_source);
//! ```
//!
//! [neorg]: https://github.com/nvim-neorg/neorg
//! [pandoc]: https://pandoc.org/

use std::collections::HashMap;

use document::DocumentBuilder;
use pandoc_types::definition::{Attr, Pandoc};
use tree_sitter::TreeCursor;

use ir::Block;

mod document;
mod extensions;
mod inlines;
mod ir;
mod lists;
mod meta;
mod quote;
mod tags;

pub use extensions::TodoSymbols;

/// The `Frontend` is the central structure of the converter.
///
/// To start using a `Frontend` first create an instance of it by calling [`Frontend::default`],
/// this will use a default configuration, in order to use a custom [`Config`] use [`Frontend::new`].
///
/// Then to convert to the pandoc representation, call [`convert`] on the `Frontend`, this will
/// output a type that can be serialized with `serde`.
///
/// The same `Frontend` instance should be used for many neorg documents if they all belong to the
/// same pandoc document, for example if generating an html document by including the result of
/// many neorg documents and stitching them together, this is because the `Frontend` keeps track of
/// some information in order to ensure for example unique identifiers between the processed files.
///
/// [`&str`]: str
/// [`convert`]: Frontend::convert
#[derive(Default)]
pub struct Frontend {
    config: Config,
    identifiers: HashMap<String, u32>,
}

impl Frontend {
    /// Creates a new `Frontend` with the provided configuration.
    pub fn new(config: Config) -> Self {
        Frontend {
            config,
            ..Default::default()
        }
    }

    /// Converts the passed neorg source code to it's pandoc representation.
    pub fn convert(&mut self, source: &str) -> Pandoc {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(tree_sitter_norg::language())
            .expect("Failed to load tree sitter grammar");

        let tree = parser.parse(source, None).expect("Failed to parse file");
        let mut cursor = tree.walk();

        let mut builder = Builder {
            source,
            cursor: &mut cursor,
            document: DocumentBuilder::default(),
            frontend: self,
        };

        builder.handle_node();

        builder.document.build()
    }

    /// Generates an unique (for a given `Frontend` instance) string that's a
    /// valid HTML5 `id` attribute value from the passed text.
    fn generate_id(&mut self, text: &str) -> String {
        // https://html.spec.whatwg.org/multipage/dom.html#the-id-attribute
        //
        // > When specified on HTML elements, the id attribute value must be unique
        // > amongst all the IDs in the element's tree and must contain at least one
        // > character. The value must not contain any ASCII whitespace.
        //
        // Also replace tildes (`~`) so that they can be used for appending the counter,
        // and other whitespace-like characthers (like tabs and newlines) because while this
        // isn't necessary for HTML5 other formats don't handle them well
        let mut base = text.replace([' ', '~', '\t', '\n'], "-");

        // If `base` was already used as an identifier a counter will be appended
        // to it so that a new unique id can be generated
        match self.identifiers.get_mut(&base) {
            Some(counter) => {
                base.push_str(&format!("~{}", *counter));
                *counter += 1;
            }
            None => {
                self.identifiers.insert(base.clone(), 0);
            }
        }

        base
    }
}

/// Holds the configuration used by a [`Frontend`].
///
/// A default configuration can be generated using the [`default`] function.
///
/// [`default`]: Config::default
#[derive(Default)]
pub struct Config {
    /// Defines the symbols to be used for neorg's TODO status extension.
    pub todo_symbols: TodoSymbols,
}

struct Builder<'builder, 'tree> {
    source: &'tree str,
    cursor: &'builder mut TreeCursor<'tree>,
    document: DocumentBuilder,
    frontend: &'tree mut Frontend,
}

impl<'builder, 'tree> Builder<'builder, 'tree> {
    fn handle_node(&mut self) {
        let node = self.cursor.node();

        log::trace!("Found node '{}'", node.kind());

        match node.kind() {
            "document" => self.handle_document(),
            "heading1" => self.handle_heading(1),
            "heading2" => self.handle_heading(2),
            "heading3" => self.handle_heading(3),
            "heading4" => self.handle_heading(4),
            "heading5" => self.handle_heading(5),
            "heading6" => self.handle_heading(6),
            "quote" => self.handle_quote(),
            "_paragraph_break" => {}
            "paragraph" => self.handle_paragraph(None),
            "ranged_tag" => self.handle_ranged_tag(),
            "ranged_verbatim_tag" => self.handle_verbatim(),
            "generic_list" => self.handle_lists(),
            kind => {
                log::error!("Unknown node: {:?}", kind)
            }
        }
    }

    fn visit_children<F>(&mut self, mut visitor: F) -> bool
    where
        F: FnMut(&mut Self),
    {
        if !self.cursor.goto_first_child() {
            return false;
        }

        loop {
            visitor(self);

            if !self.cursor.goto_next_sibling() {
                break;
            }
        }

        self.cursor.goto_parent();

        true
    }

    fn handle_document(&mut self) {
        log::debug!("Parsing document");

        self.visit_children(Self::handle_node);
    }

    fn handle_heading(&mut self, level: i32) {
        log::debug!("Parsing heading (level: {})", level);

        let node = self.cursor.node();

        let title_id = node.language().field_id_for_name("title");
        let content_id = node.language().field_id_for_name("content");
        let state_id = node.language().field_id_for_name("state");

        debug_assert!(title_id.is_some());
        debug_assert!(content_id.is_some());
        debug_assert!(state_id.is_some());

        self.visit_children(|this| {
            if this.cursor.field_id() == content_id {
                this.handle_node();
            } else if this.cursor.field_id() == title_id {
                let node = this.cursor.node();
                let mut inlines = this.document.take_inlines_collector();

                this.handle_segment(&mut inlines);

                let attr = Attr {
                    identifier: this
                        .frontend
                        .generate_id(&this.source[node.start_byte()..node.end_byte()]),
                    ..Default::default()
                };

                this.document.add_block(Block::Header(level, attr, inlines));
            } else if this.cursor.field_id() == state_id {
                this.handle_detached_ext();
            }
        });
    }

    fn handle_quote(&mut self) {
        log::debug!("Parsing quote");

        if !self.cursor.goto_first_child() {
            return;
        }

        let root = quote::QuoteBuilder::new(self).parse();

        self.document.add_block(Block::BlockQuote(root));

        self.cursor.goto_parent();
    }

    fn handle_paragraph(&mut self, blocks: Option<&mut Vec<Block>>) {
        log::debug!("Parsing paragraph");

        let mut segments = Vec::new();
        let mut segment = self.document.take_inlines_collector();

        self.visit_children(|this| {
            this.handle_segment(&mut segment);

            if !segment.is_empty() {
                let mut new_segment = Vec::new();
                std::mem::swap(&mut segment, &mut new_segment);
                segments.push(new_segment);
            }
        });

        if !segments.is_empty() {
            self.document
                .add_block_scoped(blocks, Block::Paragraph(segments));
        }
    }
}
