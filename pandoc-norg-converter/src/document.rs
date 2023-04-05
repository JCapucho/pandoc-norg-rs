use crate::ir::{convert_blocks_to_pandoc, convert_inlines_to_pandoc, Block, Inline};
use pandoc_types::definition::{Block as PandocBlock, MetaValue, Pandoc};
use std::collections::HashMap;

/// Interface for building pandoc documents.
///
/// This interface provides some extra functionality to help when building a document and ensures
/// their correct usage trough it's API.
pub struct DocumentBuilder<'source> {
    scopes: Vec<Vec<Block<'source>>>,
    metadata: HashMap<String, MetaValue>,
    inlines_collector: Vec<Inline<'source>>,
    anchors: HashMap<&'source str, &'source str>,
}

impl<'source> DocumentBuilder<'source> {
    /// Adds a new [`Block`] to the current scope
    pub fn add_block(&mut self, block: Block<'source>) {
        let scope = self.scopes.last_mut().expect("All scopes were popped");

        // Flush the inlines collector
        if !self.inlines_collector.is_empty() {
            let mut inlines = Vec::new();
            std::mem::swap(&mut self.inlines_collector, &mut inlines);
            scope.push(Block::Plain(inlines));
        }

        scope.push(block);
    }

    /// Pushes a new scope
    pub fn push_scope(&mut self) {
        self.scopes.push(Vec::new());
    }

    /// Pops the current scope returning it's blocks
    pub fn pop_scope(&mut self) -> Vec<Block<'source>> {
        self.scopes
            .pop()
            .expect("Tried to pop a non existing scope")
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

    /// Adds a new anchor definition
    ///
    /// If the anchor name was already added the value is replaced
    pub fn add_anchor(&mut self, name: &'source str, url: &'source str) {
        log::debug!("Registering anchor for {} (url: {})", name, url);
        self.anchors.insert(name, url);
    }

    /// Adds an inline to the collector.
    ///
    /// The collector stores inlines until either [`take_inlines_collector`] is called or a new
    /// block is added (to either the document or a scope).
    ///
    /// This is useful for directives which produce inlines but that are supposed to merge with the
    /// next block with inlines.
    ///
    /// [`take_inlines_collector`]: Self::take_inlines_collector
    pub fn push_inlines_collector(&mut self, inline: Inline<'source>) {
        self.inlines_collector.push(inline)
    }

    /// Returns the contents of the inline collector and resets it.
    pub fn take_inlines_collector(&mut self) -> Vec<Inline<'source>> {
        let mut inlines = Vec::new();
        std::mem::swap(&mut self.inlines_collector, &mut inlines);
        inlines
    }

    /// Returns the built document.
    pub fn build(mut self) -> Pandoc {
        debug_assert_eq!(self.scopes.len(), 1, "Only the root scope should remain");
        let root_scope = self.scopes.remove(0);

        let mut pandoc = Pandoc {
            meta: self.metadata,
            blocks: convert_blocks_to_pandoc(root_scope, &self.anchors),
        };

        // Flush the inlines collector
        if !self.inlines_collector.is_empty() {
            let inlines = convert_inlines_to_pandoc(self.inlines_collector, &self.anchors);
            pandoc.blocks.push(PandocBlock::Plain(inlines));
        }

        pandoc
    }
}

impl Default for DocumentBuilder<'_> {
    fn default() -> Self {
        Self {
            scopes: vec![Vec::new()],
            metadata: Default::default(),
            inlines_collector: Default::default(),
            anchors: Default::default(),
        }
    }
}
