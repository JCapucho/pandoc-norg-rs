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
use pandoc_types::definition::{
    Attr, Block, Cell, ColSpec, Inline, MathType, Pandoc, Row, Table, TableBody, TableHead, Target,
};
use tree_sitter::TreeCursor;

mod document;
mod extensions;
mod inlines;
mod lists;
mod meta;
mod quote;

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
        // Also replace dots (`.`) so that they can be used for appending the counter.
        let mut base = text.replace(' ', "-").replace('~', "-");

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

                let mut attr = Attr::default();
                attr.identifier = this
                    .frontend
                    .generate_id(&this.source[node.start_byte()..node.end_byte()]);

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

    fn handle_verbatim(&mut self) {
        log::debug!("Parsing verbatim");

        let mut name = "";
        let mut parameters = Vec::new();

        self.visit_children(|this| {
            let node = this.cursor.node();

            match node.kind() {
                "_prefix" | "_space" | "_line_break" | "ranged_verbatim_tag_end" => {}
                "tag_name" => {
                    let text = node
                        .utf8_text(this.source.as_bytes())
                        .expect("Invalid text");

                    name = text;
                }
                "tag_parameters" => this.handle_tag_parameters(&mut parameters),
                "ranged_verbatim_tag_content" => match name {
                    "code" => this.handle_code_block(&parameters),
                    "embed" => this.handle_embed_block(&parameters),
                    "table" => this.handle_table_block(&parameters),
                    "document.meta" => this.handle_document_meta_block(&parameters),
                    "math" => this.handle_math_block(&parameters),
                    "comment" => log::debug!("Parsing comment block"),
                    _ => log::error!("Unknown verbatim name '{}'", name),
                },

                kind => log::error!("(verbatim) unknown node: {:?}", kind),
            }
        });
    }

    fn handle_tag_parameters(&mut self, parameters: &mut Vec<&'tree str>) {
        let node = self.cursor.node();

        parameters.reserve(node.child_count());

        self.visit_children(|this| {
            let text = node
                .utf8_text(this.source.as_bytes())
                .expect("Invalid text");

            parameters.push(text);
        });
    }

    fn handle_code_block(&mut self, parameters: &[&str]) {
        log::debug!("Parsing code block");

        if parameters.len() > 1 {
            log::error!(
                "Code block expected 1 parameter received: {}",
                parameters.len()
            );
            log::error!("Extra parameters: {:?}", &parameters[1..]);
        }

        let node = self.cursor.node();
        let start_indent = node.start_position().column;
        let mut min_indent = start_indent;
        let mut size = start_indent;

        let text = node
            .utf8_text(self.source.as_bytes())
            .expect("Invalid text");

        let mut lines = text.lines();

        if let Some(line) = lines.next() {
            size += line.len();
        }

        for line in lines {
            let mut indent = 0;
            let mut all_whitespace = true;

            for char in line.chars() {
                if !char.is_whitespace() {
                    all_whitespace = false;
                    break;
                }

                indent += 1;
            }

            if !all_whitespace {
                min_indent = min_indent.min(indent);
            }

            size += line.len();
        }

        let mut content = String::with_capacity(size);

        let mut lines = text.lines();

        if let Some(line) = lines.next() {
            for _ in 0..(start_indent - min_indent) {
                content.push(' ');
            }

            content.push_str(line);
        }

        for line in lines {
            content.push('\n');
            let offset = min_indent.min(line.len());
            content.push_str(&line[offset..]);
        }

        let attr = Attr {
            classes: parameters.iter().map(ToString::to_string).collect(),
            ..Default::default()
        };
        self.document.add_block(Block::CodeBlock(attr, content))
    }

    fn handle_embed_block(&mut self, parameters: &[&str]) {
        log::debug!("Parsing embed block");

        if parameters.len() != 1 {
            log::error!(
                "Embed block expected 1 parameter received: {}",
                parameters.len()
            );
            if parameters.len() > 1 {
                log::error!("Extra parameters: {:?}", &parameters[1..]);
            }
        }

        let text = self
            .cursor
            .node()
            .utf8_text(self.source.as_bytes())
            .expect("Invalid text");

        match parameters.first().copied() {
            Some("image") => {
                let attr = Attr::default();
                let target = Target {
                    title: String::new(),
                    url: text.to_string(),
                };
                let inlines = vec![Inline::Image(attr, Vec::new(), target)];
                self.document.add_block(Block::Plain(inlines));
            }
            Some(kind) => log::error!("Unknown embed type: {}", kind),
            None => {}
        }
    }

    fn handle_table_block(&mut self, parameters: &[&str]) {
        log::debug!("Parsing table");

        if !parameters.is_empty() {
            log::error!(
                "Embed block expected 0 parameter received: {}",
                parameters.len()
            );
            log::error!("Extra parameters: {:?}", parameters);
        }

        let text = self
            .cursor
            .node()
            .utf8_text(self.source.as_bytes())
            .expect("Invalid text");

        let mut cols = 0;

        let mut parse_row = |line: &str| {
            let mut cells = Vec::new();

            for col in line.split('|') {
                let content = col.trim();
                cells.push(Cell {
                    content: vec![Block::Plain(vec![Inline::Str(content.to_string())])],
                    ..Default::default()
                });
            }

            cols = cols.max(cells.len());

            Row {
                attr: Attr::default(),
                cells,
            }
        };

        let mut head = Vec::new();
        let mut rows = Vec::new();

        let mut lines = text.lines();

        if let Some(line) = lines.next() {
            head.push(parse_row(line))
        }

        for line in lines {
            rows.push(parse_row(line))
        }

        self.document.add_block(Block::Table(Table {
            colspecs: vec![ColSpec::default(); cols],
            head: TableHead {
                rows: head,
                ..Default::default()
            },
            bodies: vec![TableBody {
                body: rows,
                ..Default::default()
            }],
            ..Default::default()
        }));
    }

    fn handle_math_block(&mut self, parameters: &[&str]) {
        log::debug!("Parsing math block");

        if !parameters.is_empty() {
            log::error!(
                "Math block expected 0 parameter received: {}",
                parameters.len()
            );
            log::error!("Extra parameters: {:?}", parameters);
        }

        let text = self
            .cursor
            .node()
            .utf8_text(self.source.as_bytes())
            .expect("Invalid text");

        self.document.add_block(Block::Para(vec![Inline::Math(
            MathType::DisplayMath,
            text.to_string(),
        )]));
    }

    fn handle_paragraph(&mut self, blocks: Option<&mut Vec<Block>>) {
        log::debug!("Parsing paragraph");

        let mut inlines = self.document.take_inlines_collector();

        let has_children = self.visit_children(|this| {
            this.handle_segment(&mut inlines);
        });

        if has_children {
            self.document.add_block_scoped(blocks, Block::Para(inlines));
        }
    }
}
