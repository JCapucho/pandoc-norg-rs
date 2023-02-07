use pandoc_types::definition::Block;

use crate::Builder;

pub(crate) struct QuoteBuilder<'a, 'builder, 'tree> {
    builder: &'a mut Builder<'builder, 'tree>,
    blocks: [Vec<Block>; 6],
    last_level: usize,
}

impl<'a, 'builder, 'tree> QuoteBuilder<'a, 'builder, 'tree> {
    pub fn new(builder: &'a mut Builder<'builder, 'tree>) -> Self {
        Self {
            builder,
            blocks: Default::default(),
            last_level: 0,
        }
    }

    pub fn parse(mut self) -> Vec<Block> {
        loop {
            let node = self.builder.cursor.node();

            match node.kind() {
                "quote1" => self.handle_quote_level(0),
                "quote2" => self.handle_quote_level(1),
                "quote3" => self.handle_quote_level(2),
                "quote4" => self.handle_quote_level(3),
                "quote5" => self.handle_quote_level(4),
                "quote6" => self.handle_quote_level(5),
                kind => log::error!("(quote) unknown node: {:?}", kind),
            }

            if !self.builder.cursor.goto_next_sibling() {
                break;
            }
        }

        let Self {
            blocks: [root, ..], ..
        } = self;

        root
    }

    fn merge_quotes(&mut self, level: usize) {
        let mut i = self.last_level;
        while i > level {
            let mut temp = Vec::new();
            std::mem::swap(&mut temp, &mut self.blocks[i]);
            self.blocks[i - 1].push(Block::BlockQuote(temp));
            i -= 1;
        }

        self.last_level = level;
    }

    fn handle_quote_level(&mut self, level: usize) {
        if !self.builder.cursor.goto_first_child() {
            return;
        }

        loop {
            let node = self.builder.cursor.node();

            match node.kind() {
                "quote1" => self.handle_quote_level(0),
                "quote2" => self.handle_quote_level(1),
                "quote3" => self.handle_quote_level(2),
                "quote4" => self.handle_quote_level(3),
                "quote5" => self.handle_quote_level(4),
                "quote6" => self.handle_quote_level(5),

                "quote1_prefix" | "quote2_prefix" | "quote3_prefix" | "quote4_prefix"
                | "quote5_prefix" | "quote6_prefix" => {}

                "paragraph" => {
                    self.merge_quotes(level);
                    self.builder.handle_paragraph(Some(&mut self.blocks[level]));
                }

                "detached_modifier_extension" => self.builder.handle_detached_ext(),

                kind => log::error!("(quote) unknown node: {:?}", kind),
            }

            if !self.builder.cursor.goto_next_sibling() {
                break;
            }
        }

        self.merge_quotes(level);

        self.builder.cursor.goto_parent();
    }
}
