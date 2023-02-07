use pandoc_types::definition::Block;

use crate::Builder;

#[derive(PartialEq)]
enum ExitCondition {
    EndOfNodes,
    LevelIsHigher,
}

impl<'builder, 'tree> Builder<'builder, 'tree> {
    pub fn handle_lists(&mut self) {
        log::debug!("Parsing list");

        if !self.cursor.goto_first_child() {
            return;
        }

        loop {
            let (list, exit) = self.build_lists_level(0);
            self.document.blocks.push(list);

            if let ExitCondition::EndOfNodes = exit {
                break;
            }
        }

        self.cursor.goto_parent();
    }

    fn build_lists_level(&mut self, level: usize) -> (Block, ExitCondition) {
        let mut items = Vec::new();
        let mut exit = ExitCondition::EndOfNodes;

        loop {
            let node = self.cursor.node();

            log::debug!("{level} {}", node.kind());

            let new_level = match node.kind() {
                "unordered_list1" => 0,
                "unordered_list2" => 1,
                "unordered_list3" => 2,
                "unordered_list4" => 3,
                "unordered_list5" => 4,
                "unordered_list6" => 5,
                kind => {
                    log::error!("(lists) unknown node: {:?}", kind);
                    if !self.cursor.goto_next_sibling() {
                        break;
                    } else {
                        continue;
                    }
                }
            };

            match level.cmp(&new_level) {
                std::cmp::Ordering::Less => {
                    let (mut list, sub_exit) = self.build_lists_level(new_level);
                    let diff = new_level - level;
                    for _ in 1..diff {
                        list = Block::BulletList(vec![vec![list]]);
                    }
                    items.push(vec![list]);

                    if sub_exit != ExitCondition::EndOfNodes {
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

        (Block::BulletList(items), exit)
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

                "unordered_list1" | "unordered_list2" | "unordered_list3" | "unordered_list4"
                | "unordered_list5" | "unordered_list6" => {
                    blocks.push(this.build_lists_level(level + 1).0)
                }

                "paragraph" => this.handle_paragraph(Some(&mut blocks)),

                kind => log::error!("(lists) unknown node: {:?}", kind),
            }
        });

        blocks
    }
}
