use clap::Parser;
use pandoc_types::definition::{
    Attr, Block, Cell, ColSpec, Inline, Pandoc, Row, Table, TableBody, TableHead, Target,
};
use std::{fs, path::PathBuf};
use tree_sitter::TreeCursor;

mod meta;
mod quote;

/// Converts a neorg file to pandoc json
#[derive(Parser, Debug)]
struct Args {
    /// Path of the neorg file to process
    file: PathBuf,
}

fn main() {
    let args = Args::parse();
    let mut builder = env_logger::Builder::new();
    builder.filter_level(log::LevelFilter::Info);
    builder.parse_default_env();
    builder.init();

    let file_contents = fs::read_to_string(args.file).expect("Failed to open neorg file");

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(tree_sitter_norg::language())
        .expect("Failed to load tree sitter grammar");

    let tree = parser
        .parse(&file_contents, None)
        .expect("Failed to parse file");
    let mut cursor = tree.walk();

    let mut builder = Builder {
        source: &file_contents,
        cursor: &mut cursor,
        document: Pandoc::default(),
    };

    builder.handle_node();

    let stdout = std::io::stdout().lock();
    serde_json::to_writer_pretty(stdout, &builder.document).expect("Failed to output to stdout");
}

struct Builder<'builder, 'tree> {
    source: &'tree str,
    cursor: &'builder mut TreeCursor<'tree>,
    document: Pandoc,
}

impl<'builder, 'tree> Builder<'builder, 'tree> {
    fn add_block(&mut self, blocks: Option<&mut Vec<Block>>, block: Block) {
        if let Some(blocks) = blocks {
            blocks.push(block);
        } else {
            self.document.blocks.push(block);
        }
    }

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
            kind => {
                log::error!("Unknown node: {:?}", kind)
            }
        }
    }

    fn handle_document(&mut self) {
        log::debug!("Parsing document");

        if !self.cursor.goto_first_child() {
            return;
        }

        loop {
            self.handle_node();

            if !self.cursor.goto_next_sibling() {
                break;
            }
        }

        self.cursor.goto_parent();
    }

    fn handle_heading(&mut self, level: i32) {
        log::debug!("Parsing heading (level: {})", level);

        let node = self.cursor.node();

        let title_id = node.language().field_id_for_name("title");
        let content_id = node.language().field_id_for_name("content");

        debug_assert!(title_id.is_some());
        debug_assert!(content_id.is_some());

        if !self.cursor.goto_first_child() {
            return;
        }

        loop {
            if self.cursor.field_id() == content_id {
                self.handle_node();
            } else if self.cursor.field_id() == title_id {
                let mut inlines = Vec::new();

                self.handle_segment(&mut inlines);

                self.document
                    .blocks
                    .push(Block::Header(level, Attr::default(), inlines));
            }

            if !self.cursor.goto_next_sibling() {
                break;
            }
        }

        self.cursor.goto_parent();
    }

    fn handle_quote(&mut self) {
        log::debug!("Parsing quote");

        if !self.cursor.goto_first_child() {
            return;
        }

        let root = quote::QuoteBuilder::new(self).parse();

        self.document.blocks.push(Block::BlockQuote(root));

        self.cursor.goto_parent();
    }

    fn handle_verbatim(&mut self) {
        log::debug!("Parsing verbatim");

        if !self.cursor.goto_first_child() {
            return;
        }

        let mut name = "";
        let mut parameters = Vec::new();

        loop {
            let node = self.cursor.node();

            match node.kind() {
                "_prefix" | "_space" | "_line_break" | "ranged_verbatim_tag_end" => {}
                "tag_name" => {
                    let text = node
                        .utf8_text(self.source.as_bytes())
                        .expect("Invalid text");

                    name = text;
                }
                "tag_parameters" => self.handle_tag_parameters(&mut parameters),
                "ranged_verbatim_tag_content" => match name {
                    "code" => self.handle_code_block(&parameters),
                    "embed" => self.handle_embed_block(&parameters),
                    "table" => self.handle_table_block(&parameters),
                    "document.meta" => self.handle_document_meta_block(&parameters),
                    "comment" => log::debug!("Parsing comment block"),
                    _ => log::error!("Unknown verbatim name '{}'", name),
                },

                kind => log::error!("(verbatim) unknown node: {:?}", kind),
            }

            if !self.cursor.goto_next_sibling() {
                break;
            }
        }

        self.cursor.goto_parent();
    }

    fn handle_tag_parameters(&mut self, parameters: &mut Vec<&'tree str>) {
        let node = self.cursor.node();

        parameters.reserve(node.child_count());

        if !self.cursor.goto_first_child() {
            return;
        }

        loop {
            let text = node
                .utf8_text(self.source.as_bytes())
                .expect("Invalid text");

            parameters.push(text);

            if !self.cursor.goto_next_sibling() {
                break;
            }
        }

        self.cursor.goto_parent();
    }

    fn handle_code_block(&mut self, parameters: &[&str]) {
        log::debug!("Parsing code block");

        if parameters.len() != 1 {
            log::error!(
                "WARN: Code block expected 1 parameter received: {}",
                parameters.len()
            );
            if parameters.len() > 1 {
                log::error!("WARN: Extra parameters: {:?}", &parameters[1..]);
            }
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
            classes: parameters.into_iter().map(ToString::to_string).collect(),
            ..Default::default()
        };
        self.document.blocks.push(Block::CodeBlock(attr, content))
    }

    fn handle_embed_block(&mut self, parameters: &[&str]) {
        log::debug!("Parsing embed block");

        if parameters.len() != 1 {
            log::error!(
                "WARN: Embed block expected 1 parameter received: {}",
                parameters.len()
            );
            if parameters.len() > 1 {
                log::error!("WARN: Extra parameters: {:?}", &parameters[1..]);
            }
        }

        let text = self
            .cursor
            .node()
            .utf8_text(self.source.as_bytes())
            .expect("Invalid text");

        match parameters.get(0).copied() {
            Some("image") => {
                let attr = Attr::default();
                let target = Target {
                    title: String::new(),
                    url: text.to_string(),
                };
                let inlines = vec![Inline::Image(attr, Vec::new(), target)];
                self.document.blocks.push(Block::Plain(inlines));
            }
            Some(kind) => log::error!("Unknown embed type: {}", kind),
            None => {}
        }
    }

    fn handle_table_block(&mut self, parameters: &[&str]) {
        log::debug!("Parsing table");

        if parameters.len() != 0 {
            log::error!(
                "WARN: Embed block expected 0 parameter received: {}",
                parameters.len()
            );
            if parameters.len() > 0 {
                log::error!("WARN: Extra parameters: {:?}", parameters);
            }
        }

        let text = self
            .cursor
            .node()
            .utf8_text(self.source.as_bytes())
            .expect("Invalid text");

        let mut cols = 0;

        let mut parse_row = |line: &str| {
            let mut cells = Vec::new();

            for col in line.split("|") {
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

        self.document.blocks.push(Block::Table(Table {
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

    fn handle_paragraph(&mut self, blocks: Option<&mut Vec<Block>>) {
        log::debug!("Parsing paragraph");

        if !self.cursor.goto_first_child() {
            return;
        }

        let mut inlines = Vec::new();

        loop {
            self.handle_segment(&mut inlines);

            if !self.cursor.goto_next_sibling() {
                break;
            }
        }

        self.add_block(blocks, Block::Para(inlines));

        self.cursor.goto_parent();
    }

    fn handle_segment(&mut self, inlines: &mut Vec<Inline>) {
        let node = self.cursor.node();

        log::trace!("Parsing segment '{}'", node.kind());

        match node.kind() {
            "paragraph_segment" => {
                if self.cursor.goto_first_child() {
                    loop {
                        self.handle_segment(inlines);

                        if !self.cursor.goto_next_sibling() {
                            break;
                        }
                    }

                    self.cursor.goto_parent();
                }
            }
            "_word" => {
                let text = node
                    .utf8_text(self.source.as_bytes())
                    .expect("Invalid text")
                    .to_string();
                inlines.push(Inline::Str(text));
            }
            "_space" => inlines.push(Inline::Space),
            "_trailing_modifier" => {
                let text = node
                    .utf8_text(self.source.as_bytes())
                    .expect("Invalid text");

                match text {
                    "~" => {}
                    modifier => log::error!("Unknown trailing modifier {}", modifier),
                }
            }
            "_line_break" => inlines.push(Inline::LineBreak),
            "verbatim" => {
                let text = self.get_delimited_modifier_text();
                inlines.push(Inline::Code(Attr::default(), text.to_string()))
            }
            "bold" => {
                let mut bold_inlines = Vec::new();

                if self.cursor.goto_first_child() {
                    loop {
                        let node = self.cursor.node();

                        match node.kind() {
                            "_open" | "_close" => {}
                            _ => self.handle_segment(&mut bold_inlines),
                        }

                        if !self.cursor.goto_next_sibling() {
                            break;
                        }
                    }

                    self.cursor.goto_parent();
                }

                inlines.push(Inline::Strong(bold_inlines))
            }
            "link" => inlines.push(self.handle_link()),
            "escape_sequence" => {
                let token_id = node.language().field_id_for_name("token");

                if self.cursor.goto_first_child() {
                    loop {
                        if self.cursor.field_id() == token_id {
                            let text = self
                                .cursor
                                .node()
                                .utf8_text(self.source.as_bytes())
                                .expect("Invalid text")
                                .to_string();

                            inlines.push(Inline::Str(text));
                        }

                        if !self.cursor.goto_next_sibling() {
                            break;
                        }
                    }

                    self.cursor.goto_parent();
                }
            }
            kind => {
                log::error!("Unknown segment: {:?}", kind);
            }
        }
    }

    fn get_delimited_modifier_text(&mut self) -> &str {
        let node = self.cursor.node();
        let mut start = node.start_byte();
        let mut end = node.end_byte();

        if self.cursor.goto_first_child() {
            loop {
                let node = self.cursor.node();

                match node.kind() {
                    "_open" => start = self.cursor.node().end_byte(),
                    "_close" => end = self.cursor.node().start_byte(),
                    _ => {}
                }

                if !self.cursor.goto_next_sibling() {
                    break;
                }
            }

            self.cursor.goto_parent();
        }

        &self.source[start..end]
    }

    fn handle_link(&mut self) -> Inline {
        let mut text_inlines = Vec::new();
        let mut target = Target {
            url: String::new(),
            title: String::new(),
        };

        if self.cursor.goto_first_child() {
            loop {
                let node = self.cursor.node();

                match node.kind() {
                    "link_description" => self.handle_link_description(&mut text_inlines),
                    "link_location" => {
                        match node.child_by_field_name("type").map(|node| node.kind()) {
                            Some("link_target_url") => {}
                            Some(ty) => log::error!("Unknown link type: {}", ty),
                            None => log::error!("Link with no type"),
                        }

                        if let Some(text_node) = node.child_by_field_name("text") {
                            target.url = text_node
                                .utf8_text(self.source.as_bytes())
                                .expect("Invalid text")
                                .to_string();
                        }
                    }
                    link_child => log::error!("Unknown link child: {}", link_child),
                }

                if !self.cursor.goto_next_sibling() {
                    break;
                }
            }

            self.cursor.goto_parent();
        }

        Inline::Link(Attr::default(), text_inlines, target)
    }

    fn handle_link_description(&mut self, inlines: &mut Vec<Inline>) {
        let node = self.cursor.node();

        let text_id = node.language().field_id_for_name("text");

        if self.cursor.goto_first_child() {
            loop {
                if self.cursor.field_id() == text_id {
                    if self.cursor.goto_first_child() {
                        loop {
                            self.handle_segment(inlines);

                            if !self.cursor.goto_next_sibling() {
                                break;
                            }
                        }

                        self.cursor.goto_parent();
                    }
                }

                if !self.cursor.goto_next_sibling() {
                    break;
                }
            }
            self.cursor.goto_parent();
        }
    }
}
