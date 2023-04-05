use crate::ir::Block;
use crate::Builder;

impl<'builder, 'source> Builder<'builder, 'source> {
    pub fn handle_definition_list(&mut self) {
        log::debug!("Parsing definition list");

        self.visit_children(Self::handle_definition);
    }

    fn handle_definition(&mut self) {
        log::debug!("Parsing definition");

        let mut entries = Vec::new();
        let mut inlines = Vec::new();

        self.document.push_scope();

        self.visit_children(|this| {
            if this.cursor.field_id() == this.field_ids.content {
                this.handle_node();
            } else if this.cursor.field_id() == this.field_ids.title {
                if !inlines.is_empty() {
                    let mut old_inlines = Vec::new();
                    std::mem::swap(&mut inlines, &mut old_inlines);
                    entries.push((old_inlines, this.document.pop_scope()));
                    this.document.push_scope();
                }

                inlines.append(&mut this.document.take_inlines_collector());
                this.handle_segment(&mut inlines);
            } else if this.cursor.field_id() == this.field_ids.state {
                this.handle_detached_ext();
            }
        });

        let last_blocks = self.document.pop_scope();
        if !inlines.is_empty() {
            entries.push((inlines, last_blocks));
        }

        self.document.add_block(Block::DefinitionList(entries));
    }
}
