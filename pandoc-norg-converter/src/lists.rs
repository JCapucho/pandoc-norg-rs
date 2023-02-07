use pandoc_types::definition::Block;

use crate::Builder;

/// The list type currently being processed.
#[derive(PartialEq, Clone, Copy)]
enum ListType {
    /// No list node was processed yet so the list type is unknown.
    Unknown,
    /// The current list being processed is ordered.
    Ordered,
    /// The current list being processed is unordered.
    Unordered,
}

/// Defines the reason why [`Builder::build_lists_level`] stopped processing new nodes for lists.
#[derive(PartialEq)]
enum ExitCondition {
    /// There are no more nodes to explore.
    EndOfNodes,
    /// A list node was found but it's nesting level is lower than the level currently being
    /// processed.
    LevelIsHigher,
    /// A list node was found but it's of a different type for the type currently being processed.
    TypeMismatch,
}

/// The results of a call to [`Builder::build_list_level`].
struct BuildListsResult {
    /// The built list as a block.
    block: Block,
    /// The type of list that was built.
    list_type: ListType,
    /// The reason why the build process stopped.
    exit: ExitCondition,
}

impl<'builder, 'tree> Builder<'builder, 'tree> {
    pub fn handle_lists(&mut self) {
        log::debug!("Parsing list");

        if !self.cursor.goto_first_child() {
            return;
        }

        loop {
            let res = self.build_lists_level(0);
            self.document.add_block(res.block);

            if let ExitCondition::EndOfNodes = res.exit {
                break;
            }
        }

        self.cursor.goto_parent();
    }

    fn build_lists_level(&mut self, level: usize) -> BuildListsResult {
        let mut items = Vec::new();
        let mut exit = ExitCondition::EndOfNodes;
        let mut list_type = ListType::Unknown;

        loop {
            let node = self.cursor.node();

            let (new_level, new_type) = match node.kind() {
                "unordered_list1" => (0, ListType::Unordered),
                "unordered_list2" => (1, ListType::Unordered),
                "unordered_list3" => (2, ListType::Unordered),
                "unordered_list4" => (3, ListType::Unordered),
                "unordered_list5" => (4, ListType::Unordered),
                "unordered_list6" => (5, ListType::Unordered),

                "ordered_list1" => (0, ListType::Ordered),
                "ordered_list2" => (1, ListType::Ordered),
                "ordered_list3" => (2, ListType::Ordered),
                "ordered_list4" => (3, ListType::Ordered),
                "ordered_list5" => (4, ListType::Ordered),
                "ordered_list6" => (5, ListType::Ordered),

                kind => {
                    log::error!("(lists) unknown node: {:?}", kind);
                    if !self.cursor.goto_next_sibling() {
                        break;
                    } else {
                        continue;
                    }
                }
            };

            match (new_type, list_type) {
                (_, ListType::Unknown) => list_type = new_type,
                (x, y) if x != y => {
                    exit = ExitCondition::TypeMismatch;
                    break;
                }
                _ => {}
            }

            match level.cmp(&new_level) {
                std::cmp::Ordering::Less => {
                    let res = self.build_lists_level(new_level);
                    let mut list = res.block;
                    let diff = new_level - level;
                    for _ in 1..diff {
                        list = list_from_type(res.list_type, vec![vec![list]])
                    }
                    items.push(vec![list]);

                    if res.exit != ExitCondition::EndOfNodes {
                        continue;
                    }
                }
                std::cmp::Ordering::Equal => items.push(self.handle_list_content(level)),
                std::cmp::Ordering::Greater => {
                    exit = ExitCondition::LevelIsHigher;
                    break;
                }
            };

            if !self.cursor.goto_next_sibling() {
                break;
            }
        }

        BuildListsResult {
            block: list_from_type(list_type, items),
            list_type,
            exit,
        }
    }

    fn handle_list_content(&mut self, level: usize) -> Vec<Block> {
        let mut blocks = Vec::new();

        self.visit_children(|this| {
            let node = this.cursor.node();

            match node.kind() {
                "unordered_list1_prefix"
                | "unordered_list2_prefix"
                | "unordered_list3_prefix"
                | "unordered_list4_prefix"
                | "unordered_list5_prefix"
                | "unordered_list6_prefix" => {}

                "ordered_list1_prefix"
                | "ordered_list2_prefix"
                | "ordered_list3_prefix"
                | "ordered_list4_prefix"
                | "ordered_list5_prefix"
                | "ordered_list6_prefix" => {}

                "unordered_list1" | "unordered_list2" | "unordered_list3" | "unordered_list4"
                | "unordered_list5" | "unordered_list6" | "ordered_list1" | "ordered_list2"
                | "ordered_list3" | "ordered_list4" | "ordered_list5" | "ordered_list6" => {
                    blocks.push(this.build_lists_level(level + 1).block)
                }

                "paragraph" => this.handle_paragraph(Some(&mut blocks)),

                "detached_modifier_extension" => this.handle_detached_ext(),

                kind => log::error!("(lists) unknown node: {:?}", kind),
            }
        });

        blocks
    }
}

/// Constructs a list block from a set of items and the list type.
fn list_from_type(list_type: ListType, items: Vec<Vec<Block>>) -> Block {
    match list_type {
        ListType::Unknown => Block::Null,
        ListType::Ordered => Block::OrderedList(Default::default(), items),
        ListType::Unordered => Block::BulletList(items),
    }
}
