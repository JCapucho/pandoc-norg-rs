use crate::Builder;
use pandoc_types::definition::{
    Attr, Block, Cell, ColSpec, Inline, MathType, Row, Table, TableBody, TableHead, Target,
};

impl<'builder, 'tree> Builder<'builder, 'tree> {
    pub fn handle_ranged_tag(&mut self) {
        log::debug!("Parsing ranged tag");

        let mut name = "";
        let mut parameters = Vec::new();

        self.visit_children(|this| {
            let node = this.cursor.node();

            match node.kind() {
                "_prefix" | "_space" | "_line_break" | "ranged_tag_end" => {}
                "tag_name" => {
                    let text = node
                        .utf8_text(this.source.as_bytes())
                        .expect("Invalid text");

                    name = text;
                }

                "tag_parameters" => this.handle_tag_parameters(&mut parameters),

                "ranged_tag_content" => match name {
                    "example" => this.handle_example_block(&parameters),
                    _ => log::error!("Unknown ranged tag name '{}'", name),
                },

                kind => log::error!("(ranged_tag) unknown node: {:?}", kind),
            }
        });
    }

    pub fn handle_verbatim(&mut self) {
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

    fn handle_example_block(&mut self, parameters: &[&str]) {
        log::debug!("Parsing example block");

        if !parameters.is_empty() {
            log::error!(
                "Example block expected 0 parameter received: {}",
                parameters.len()
            );
            log::error!("Extra parameters: {:?}", parameters);
        }

        let content = self.code_content();
        let attr = Attr {
            classes: vec![String::from("norg")],
            ..Default::default()
        };
        self.document.add_block(Block::CodeBlock(attr, content))
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

        let content = self.code_content();
        let attr = Attr {
            classes: parameters.iter().map(ToString::to_string).collect(),
            ..Default::default()
        };
        self.document.add_block(Block::CodeBlock(attr, content))
    }

    fn code_content(&self) -> String {
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

        content
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
}
