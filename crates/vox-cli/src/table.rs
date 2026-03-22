//! Table rendering utilities for vox-cli.
//! Outputs to stdout; respects --json and --color.

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};

/// A wrapper around `comfy_table::Table` for consistent CLI output.
pub struct OutputTable {
    table: Table,
}

impl OutputTable {
    /// Create a new table with the given column headers.
    pub fn new(headers: &[&str]) -> Self {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic);

        if crate::diagnostics::should_color_stdout() {
            table.set_header(
                headers
                    .iter()
                    .map(|h| Cell::new(h).add_attribute(Attribute::Bold).fg(Color::Cyan)),
            );
        } else {
            table.set_header(headers);
        }

        Self { table }
    }

    /// Add a row of data to the table.
    pub fn add_row(&mut self, cells: Vec<String>) -> &mut Self {
        self.table.add_row(cells);
        self
    }

    /// Print the table to stdout.
    pub fn print(self) {
        if crate::diagnostics::should_color_stdout() {
            println!("{}", self.table);
        } else {
            // Use a plain ASCII-friendly preset if coloring/TTY is disabled
            let mut plain_table = self.table;
            plain_table.load_preset(comfy_table::presets::ASCII_MARKDOWN);
            println!("{}", plain_table);
        }
    }
}
