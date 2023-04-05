use crate::ir::Inline;
use crate::Builder;

impl<'builder, 'source> Builder<'builder, 'source>
where
    'source: 'builder,
{
    /// Handles a paragraph segment element or any children of it.
    ///
    /// The processed items are added to the provided inlines vector and the function.
    pub fn handle_segment(&mut self, inlines: &mut Vec<Inline<'source>>) {
        let node = self.cursor.node();

        log::trace!("Parsing segment '{}'", node.kind());

        match node.kind() {
            "paragraph_segment" => {
                self.visit_children(|this| {
                    this.handle_segment(inlines);
                });
            }
            "_word" => {
                let text = node
                    .utf8_text(self.source.as_bytes())
                    .expect("Invalid text");
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
            "_line_break" => {}
            "escape_sequence" => {
                self.visit_children(|this| {
                    if this.cursor.field_id() != this.field_ids.token {
                        return;
                    }

                    let text = this
                        .cursor
                        .node()
                        .utf8_text(this.source.as_bytes())
                        .expect("Invalid text");

                    inlines.push(Inline::Str(text));
                });
            }
            "link" => inlines.push(self.handle_link(false)),
            "anchor_declaration" => inlines.push(self.handle_link(true)),
            "anchor_definition" => inlines.push(self.handle_link(true)),
            // Attached modifiers
            "bold" => inlines.push(Inline::Strong(self.handle_attached_modifier_content())),
            "underline" => inlines.push(Inline::Underline(self.handle_attached_modifier_content())),
            "italic" => inlines.push(Inline::Emph(self.handle_attached_modifier_content())),
            "strikethrough" => {
                inlines.push(Inline::Strikeout(self.handle_attached_modifier_content()))
            }
            "superscript" => {
                inlines.push(Inline::Superscript(self.handle_attached_modifier_content()))
            }
            "subscript" => inlines.push(Inline::Subscript(self.handle_attached_modifier_content())),
            "verbatim" => {
                let text = self.get_delimited_modifier_text();
                inlines.push(Inline::Code(text))
            }
            "inline_math" => {
                let text = self.get_delimited_modifier_text();
                inlines.push(Inline::Math(text))
            }
            // Null modifier
            "inline_comment" => {}
            kind => {
                log::error!("Unknown segment: {:?}", kind);
            }
        }
    }

    fn handle_attached_modifier_content(&mut self) -> Vec<Inline<'source>> {
        let mut inlines = Vec::new();

        self.visit_children(|this| {
            let node = this.cursor.node();

            match node.kind() {
                "_open" | "_close" | "free_form_open" | "free_form_close" => {}
                _ => this.handle_segment(&mut inlines),
            }
        });

        inlines
    }

    fn get_delimited_modifier_text(&mut self) -> &'source str {
        let node = self.cursor.node();
        let mut start = node.start_byte();
        let mut end = node.end_byte();

        self.visit_children(|this| {
            let node = this.cursor.node();

            match node.kind() {
                "_open" => start = start.max(this.cursor.node().end_byte()),
                "_close" => end = end.min(this.cursor.node().start_byte()),
                "free_form_open" => start = start.max(this.cursor.node().end_byte()),
                "free_form_close" => end = end.min(this.cursor.node().start_byte()),
                _ => log::trace!("Node '{}' inside verbatim", node.kind()),
            }
        });

        &self.source[start..end]
    }

    fn handle_link(&mut self, is_anchor: bool) -> Inline<'source> {
        let mut has_description = false;
        let mut text_inlines = Vec::new();

        let mut has_url = false;
        let mut anchor_name = "";
        let mut anchor_url = "";

        self.visit_children(|this| {
            let node = this.cursor.node();

            match node.kind() {
                "link_description" => {
                    has_description = true;
                    this.handle_link_description(&mut text_inlines);
                    anchor_name = node
                        .utf8_text(this.source.as_bytes())
                        .expect("Invalid text");
                }
                "link_location" => {
                    match node.child_by_field_name("type").map(|node| node.kind()) {
                        Some("link_target_url") => {}
                        Some("link_target_external_file") => {}
                        Some(ty) => log::error!("Unknown link type: {}", ty),
                        None => log::error!("Link with no type"),
                    }

                    if let Some(text_node) = node.child_by_field_name("text") {
                        anchor_url = text_node
                            .utf8_text(this.source.as_bytes())
                            .expect("Invalid text");
                    }

                    has_url = true;
                }
                link_child => log::error!("Unknown link child: {}", link_child),
            }
        });

        if is_anchor && has_url {
            self.document.add_anchor(anchor_name, anchor_url);
        }

        if !has_description {
            text_inlines.push(Inline::Str(anchor_url));
        }

        match is_anchor {
            true => Inline::Anchor(text_inlines, anchor_name),
            false => Inline::Link(text_inlines, anchor_url),
        }
    }

    fn handle_link_description(&mut self, inlines: &mut Vec<Inline<'source>>) {
        self.visit_children(|this| {
            if this.cursor.field_id() != this.field_ids.text {
                return;
            }

            this.visit_children(|this| {
                this.handle_segment(inlines);
            });
        });
    }
}
