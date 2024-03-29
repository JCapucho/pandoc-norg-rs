use crate::ir::Inline;
use crate::Builder;
#[cfg(serde)]
use serde::Deserialize;

/// Contains the symbols used to represent neorg's TODO status extension.
///
/// A default set of symbols can be obtained through the [`default`] function, the
/// default symbol for each status is presented in each field's specific documentation
/// (WARNING: No guarantees are made of the stability of these symbols across versions and changing
/// them is not considered a semver breaking change).
///
/// [`default`]: TodoSymbols::default
#[derive(Debug)]
#[cfg_attr(serde, derive(Deserialize))]
#[cfg_attr(serde, serde(default))]
pub struct TodoSymbols {
    /// Task put down/cancelled `(_)` (default: ❌)
    pub cancelled: String,
    /// Task done `(x)` (default: ✅)
    pub done: String,
    /// Task on hold `(=)` (default: 🛑)
    pub on_hold: String,
    /// Task in-progress/pending `(-)` (default: ⏳)
    pub pending: String,
    /// Task recurring `(+)` (default: 🔁)
    pub recurring: String,
    /// Task needs further input/clarification `(?)` (default: ❓)
    pub uncertain: String,
    /// Task undone `( )` (default: ⬜)
    pub undone: String,
    /// Task urgent `(!)` (default: ❗)
    pub urgent: String,
}

impl Default for TodoSymbols {
    fn default() -> Self {
        Self {
            cancelled: String::from("❌"),
            done: String::from("✅"),
            on_hold: String::from("🛑"),
            pending: String::from("⏳"),
            recurring: String::from("🔁"),
            uncertain: String::from("❓"),
            undone: String::from("⬜"),
            urgent: String::from("❗"),
        }
    }
}

impl<'builder, 'source> Builder<'builder, 'source>
where
    'source: 'builder,
{
    pub fn handle_detached_ext(&mut self) {
        self.visit_children(|this| {
            let node = this.cursor.node();

            match node.kind() {
                "_begin" | "_end" | "_delimiter" => {}

                "todo_item_cancelled"
                | "todo_item_done"
                | "todo_item_on_hold"
                | "todo_item_pending"
                | "todo_item_recurring"
                | "todo_item_uncertain"
                | "todo_item_undone"
                | "todo_item_urgent" => this.add_todo_status(node.kind()),
                kind => log::error!("Unknown detached modifier extension: {kind}"),
            }
        });
    }

    fn add_todo_status(&mut self, status: &str) {
        let todo_symbols = &self.config.todo_symbols;
        let icon = match status {
            "todo_item_cancelled" => todo_symbols.cancelled.as_str(),
            "todo_item_done" => todo_symbols.done.as_str(),
            "todo_item_on_hold" => todo_symbols.on_hold.as_str(),
            "todo_item_pending" => todo_symbols.pending.as_str(),
            "todo_item_recurring" => todo_symbols.recurring.as_str(),
            "todo_item_uncertain" => todo_symbols.uncertain.as_str(),
            "todo_item_undone" => todo_symbols.undone.as_str(),
            "todo_item_urgent" => todo_symbols.urgent.as_str(),
            status => return log::error!("Unknown todo status: {status}"),
        };

        self.document.push_inlines_collector(Inline::Str(icon));
    }
}
