use std::panic;

use crate::ir::{Block, Cell};
use crate::Builder;

#[derive(Debug, PartialEq)]
pub enum TableParsingError {
    InvalidLocation,
}

type TableLocation = (usize, usize);

impl<'builder, 'source> Builder<'builder, 'source> {
    pub fn handle_table(&mut self) {
        log::debug!("Parsing table");

        let mut rows = Vec::new();
        let mut num_cols = 0;

        self.visit_children(|this| {
            let kind = this.cursor.node().kind();

            match kind {
                "single_table_cell" => {
                    let (row_idx, col_idx, cell) = this.handle_single_cell();

                    while rows.len() <= row_idx {
                        rows.push(Vec::new());
                    }

                    let row = &mut rows[row_idx];

                    while row.len() <= col_idx {
                        row.push(Cell { blocks: Vec::new() });
                    }

                    row[col_idx] = cell;

                    num_cols = num_cols.max(col_idx + 1);
                }
                _ => log::error!("Unknown node: {:?}", kind),
            }
        });

        self.document
            .add_block(Block::Table(num_cols, Vec::new(), rows))
    }

    fn handle_single_cell(&mut self) -> (usize, usize, Cell<'source>) {
        log::trace!("Parsing table single cell");

        let mut blocks = Vec::new();
        let mut row = 0;
        let mut col = 0;

        self.visit_children(|this| {
            let id = this.cursor.field_id();
            if id == this.field_ids.title {
                let node = this.cursor.node();
                let text = &this.source[node.start_byte()..node.end_byte()];

                (row, col) = parse_table_location(text).unwrap();
            } else if id == this.field_ids.content {
                this.document.push_scope();
                this.handle_paragraph();
                blocks = this.document.pop_scope();
            } else {
                match this.cursor.node().kind() {
                    "single_table_cell_prefix" | "_intersecting_modifier" => {}
                    kind => log::error!("(table) unknown node: {:?}", kind),
                };
            }
        });

        return (row, col, Cell { blocks });
    }
}

fn consume_while(input: &str, mut predicate: impl FnMut(char) -> bool) -> (&str, &str) {
    let idx = input.find(|c| !predicate(c)).unwrap_or(input.len());
    input.split_at(idx)
}

fn parse_row(row: &str) -> usize {
    let mut accum = 0;

    for char in row.chars() {
        accum *= 26;
        accum += match char {
            'A' => 1,
            'B' => 2,
            'C' => 3,
            'D' => 4,
            'E' => 5,
            'F' => 6,
            'G' => 7,
            'H' => 8,
            'I' => 9,
            'J' => 10,
            'K' => 11,
            'L' => 12,
            'M' => 13,
            'N' => 14,
            'O' => 15,
            'P' => 16,
            'Q' => 17,
            'R' => 18,
            'S' => 19,
            'T' => 20,
            'U' => 21,
            'V' => 22,
            'W' => 23,
            'X' => 24,
            'Y' => 25,
            'Z' => 26,
            _ => panic!("Invalid row char {}", char),
        };
    }

    accum - 1
}

fn parse_table_location(loc: &str) -> Result<TableLocation, TableParsingError> {
    #[derive(PartialEq, Debug)]
    enum ParsingSM {
        Start,
        Row,
        Column,
    }

    let mut state = ParsingSM::Start;
    let mut parsing = loc;

    let mut row = 0;
    let mut col = 0;

    loop {
        let Some(char) = parsing.chars().next() else {
            break;
        };

        match char {
            'A'..='Z' => {
                if state != ParsingSM::Start {
                    return Err(TableParsingError::InvalidLocation);
                }

                let (matched, rest) = consume_while(parsing, |c| ('A'..='Z').contains(&c));
                parsing = rest;

                row = parse_row(matched);

                state = ParsingSM::Row;
            }
            '1'..='9' => {
                if state != ParsingSM::Row {
                    return Err(TableParsingError::InvalidLocation);
                }

                let (matched, rest) = consume_while(parsing, |c| ('0'..='9').contains(&c));
                parsing = rest;

                col = usize::from_str_radix(matched, 10)
                    .map_err(|_| TableParsingError::InvalidLocation)?
                    - 1;

                state = ParsingSM::Column;
            }
            _ => return Err(TableParsingError::InvalidLocation),
        }
    }

    match state {
        ParsingSM::Start | ParsingSM::Row => Err(TableParsingError::InvalidLocation),
        ParsingSM::Column => Ok((row, col)),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_row, parse_table_location, TableParsingError};

    #[test]
    fn test_parse_row() {
        assert_eq!(parse_row("C"), 2);
        assert_eq!(parse_row("F"), 5);
        assert_eq!(parse_row("AC"), 28);
        assert_eq!(parse_row("ABC"), 730);
    }

    #[test]
    fn test_parse_table_location() {
        assert_eq!(parse_table_location("C1"), Ok((2, 0)));
        assert_eq!(parse_table_location("AC21"), Ok((28, 20)));
        assert_eq!(
            parse_table_location("1C"),
            Err(TableParsingError::InvalidLocation)
        );
        assert_eq!(
            parse_table_location("C1C"),
            Err(TableParsingError::InvalidLocation)
        );
        assert_eq!(
            parse_table_location(".?;"),
            Err(TableParsingError::InvalidLocation)
        );
    }
}
