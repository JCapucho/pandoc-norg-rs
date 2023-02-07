use pandoc_types::definition::{Block, Inline, MetaValue, Pandoc};

#[derive(Default)]
pub struct DocumentBuilder {
    document: Pandoc,
}

impl DocumentBuilder {
    pub fn add_block(&mut self, block: Block) {
        self.add_block_scoped(None, block);
    }

    pub fn add_block_scoped(&mut self, blocks: Option<&mut Vec<Block>>, block: Block) {
        let source = if let Some(blocks) = blocks {
            blocks
        } else {
            &mut self.document.blocks
        };

        source.push(block);
    }

    pub fn extend_meta<I>(&mut self, meta: I)
    where
        I: IntoIterator<Item = (String, MetaValue)>,
    {
        self.document.meta.extend(meta);
    }

    pub fn build(self) -> Pandoc {
        self.document
    }
}
