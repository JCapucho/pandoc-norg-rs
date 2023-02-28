use crate::ir::{convert_blocks_to_pandoc, convert_inlines_to_pandoc, Block, Inline};
use pandoc_types::definition::{Block as PandocBlock, MetaValue, Pandoc};
use std::collections::HashMap;

/// Interface for building pandoc documents.
///
/// This interface provides some extra functionality to help when building a document and ensures
/// their correct usage trough it's API.
#[derive(Default)]
pub struct DocumentBuilder {
    blocks: Vec<Block>,
    metadata: HashMap<String, MetaValue>,
    inlines_collector: Vec<Inline>,
    anchors: HashMap<String, String>,
}

impl DocumentBuilder {
    /// Adds a new [`Block`] to the document
    pub fn add_block(&mut self, block: Block) {
        self.add_block_scoped(None, block);
    }

    /// Adds a new [`Block`] to the passed scope, or if it's [`None`] to the document.
    ///
    /// [`None`]: Option::None
    pub fn add_block_scoped(&mut self, scope: Option<&mut Vec<Block>>, block: Block) {
        let sink = if let Some(blocks) = scope {
            blocks
        } else {
            &mut self.blocks
        };

        // Flush the inlines collector
        if !self.inlines_collector.is_empty() {
            let mut inlines = Vec::new();
            std::mem::swap(&mut self.inlines_collector, &mut inlines);
            sink.push(Block::Plain(inlines));
        }

        sink.push(block);
    }

    /// Extends the metadata of the document with the provided values.
    ///
    /// If a given key was already added to the metadata then it's value is replaced
    pub fn extend_meta<I>(&mut self, meta: I)
    where
        I: IntoIterator<Item = (String, MetaValue)>,
    {
        self.metadata.extend(meta);
    }

    /// Adds an inline to the collector.
    ///
    /// The collector stores inlines until either [`take_inlines_collector`] is called or a new
    /// block is added (to either the document or a scope).
    ///
    /// This is useful for directives which produce inlines but that are supposed to merge with the
    /// next block with inlines.
    pub fn push_inlines_collector(&mut self, inline: Inline) {
        self.inlines_collector.push(inline)
    }

    /// Returns the contents of the inline collector and resets it.
    pub fn take_inlines_collector(&mut self) -> Vec<Inline> {
        let mut inlines = Vec::new();
        std::mem::swap(&mut self.inlines_collector, &mut inlines);
        inlines
    }

    /// Returns the built document.
    pub fn build(self) -> Pandoc {
        let mut pandoc = Pandoc {
            meta: self.metadata,
            blocks: convert_blocks_to_pandoc(self.blocks, &self.anchors),
        };

        // Flush the inlines collector
        if !self.inlines_collector.is_empty() {
            let inlines = convert_inlines_to_pandoc(self.inlines_collector, &self.anchors);
            pandoc.blocks.push(PandocBlock::Plain(inlines));
        }

        pandoc
    }
}
