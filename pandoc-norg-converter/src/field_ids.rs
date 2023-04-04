//! Helper module for dealing and caching treesitter field name ids

use tree_sitter::Tree;

/// Structure for cahcing commonly used treesitter field ids
pub struct FieldIds {
    pub title: Option<u16>,
    pub content: Option<u16>,
    pub state: Option<u16>,
    pub token: Option<u16>,
    pub text: Option<u16>,
}

impl FieldIds {
    /// Constructs a new cache from the a built tree
    pub fn new(tree: &Tree) -> Self {
        let title = tree.language().field_id_for_name("title");
        let content = tree.language().field_id_for_name("content");
        let state = tree.language().field_id_for_name("state");
        let token = tree.language().field_id_for_name("token");
        let text = tree.language().field_id_for_name("text");

        debug_assert!(title.is_some());
        debug_assert!(content.is_some());
        debug_assert!(state.is_some());
        debug_assert!(token.is_some());
        debug_assert!(text.is_some());

        FieldIds {
            title,
            content,
            state,
            token,
            text,
        }
    }
}
