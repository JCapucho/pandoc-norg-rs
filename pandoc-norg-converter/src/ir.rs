use std::collections::HashMap;

use pandoc_types::definition::{
    Attr, Block as PandocBlock, Cell as PandocCell, ColSpec, Inline as PandocInline, MathType,
    Row as PandocRow, Table, TableBody, TableHead, Target,
};

#[derive(Debug)]
pub enum Inline<'source> {
    Space,
    Str(&'source str),

    Emph(Vec<Inline<'source>>),
    Strong(Vec<Inline<'source>>),
    Underline(Vec<Inline<'source>>),
    Strikeout(Vec<Inline<'source>>),

    Subscript(Vec<Inline<'source>>),
    Superscript(Vec<Inline<'source>>),

    Code(&'source str),
    Math(&'source str),

    Link(Vec<Inline<'source>>, &'source str),
    Anchor(Vec<Inline<'source>>, &'source str),

    Image(&'source str),
}

impl<'source> Inline<'source> {
    pub fn into_pandoc(self, anchors: &HashMap<&str, &str>) -> PandocInline {
        match self {
            Inline::Space => PandocInline::Space,
            Inline::Str(str) => PandocInline::Str(str.to_string()),
            Inline::Emph(inlines) => {
                PandocInline::Emph(convert_inlines_to_pandoc(inlines, anchors))
            }
            Inline::Strong(inlines) => {
                PandocInline::Strong(convert_inlines_to_pandoc(inlines, anchors))
            }
            Inline::Underline(inlines) => {
                PandocInline::Underline(convert_inlines_to_pandoc(inlines, anchors))
            }
            Inline::Strikeout(inlines) => {
                PandocInline::Strikeout(convert_inlines_to_pandoc(inlines, anchors))
            }
            Inline::Subscript(inlines) => {
                PandocInline::Subscript(convert_inlines_to_pandoc(inlines, anchors))
            }
            Inline::Superscript(inlines) => {
                PandocInline::Superscript(convert_inlines_to_pandoc(inlines, anchors))
            }
            Inline::Code(str) => PandocInline::Code(Attr::default(), str.to_string()),
            Inline::Math(str) => PandocInline::Math(MathType::InlineMath, str.to_string()),
            Inline::Link(inlines, url) => PandocInline::Link(
                Attr::default(),
                convert_inlines_to_pandoc(inlines, anchors),
                Target {
                    url: url.to_string(),
                    title: String::new(),
                },
            ),
            Inline::Anchor(inlines, id) => {
                let target = Target {
                    url: anchors.get(id).unwrap_or_else(|| &"").to_string(),
                    title: String::new(),
                };
                PandocInline::Link(
                    Attr::default(),
                    convert_inlines_to_pandoc(inlines, anchors),
                    target,
                )
            }
            Inline::Image(url) => {
                let attr = Attr::default();
                PandocInline::Image(
                    attr,
                    Vec::new(),
                    Target {
                        url: url.to_string(),
                        title: String::new(),
                    },
                )
            }
        }
    }
}

type Row<'source> = Vec<Cell<'source>>;
type ParagraphSegment<'source> = Vec<Inline<'source>>;

#[derive(Debug)]
pub struct Cell<'source> {
    pub blocks: Vec<Block<'source>>,
}

#[derive(Debug)]
pub struct ListEntry<'source> {
    pub blocks: Vec<Block<'source>>,
}

#[derive(Debug)]
pub enum Block<'source> {
    Null,

    Plain(ParagraphSegment<'source>),
    Paragraph(Vec<ParagraphSegment<'source>>),
    Header(i32, Attr, ParagraphSegment<'source>),
    BlockQuote(Vec<Block<'source>>),

    MathBlock(String),
    CodeBlock(Option<&'source str>, String),

    Table(usize, Row<'source>, Vec<Row<'source>>),

    BulletList(Vec<ListEntry<'source>>),
    OrderedList(Vec<ListEntry<'source>>),
    DefinitionList(Vec<(ParagraphSegment<'source>, Vec<Block<'source>>)>),
}

impl<'source> Block<'source> {
    pub fn into_pandoc(self, anchors: &HashMap<&str, &str>) -> PandocBlock {
        match self {
            Block::Null => PandocBlock::Null,
            Block::Plain(segment) => {
                let inlines = convert_inlines_to_pandoc(segment, anchors);

                PandocBlock::Plain(inlines)
            }
            Block::Paragraph(segments) => {
                let mut inlines = Vec::new();
                let mut segments = segments.into_iter();

                if let Some(segment) = segments.next() {
                    inlines.extend(convert_inlines_to_pandoc(segment, anchors));
                }

                for segment in segments {
                    inlines.push(PandocInline::Space);
                    inlines.extend(convert_inlines_to_pandoc(segment, anchors));
                }

                PandocBlock::Para(inlines)
            }
            Block::Header(level, attr, segment) => {
                let inlines = convert_inlines_to_pandoc(segment, anchors);

                PandocBlock::Header(level, attr, inlines)
            }
            Block::BlockQuote(blocks) => {
                let blocks = convert_blocks_to_pandoc(blocks, anchors);
                PandocBlock::BlockQuote(blocks)
            }
            Block::CodeBlock(language, code) => {
                let attr = Attr {
                    classes: language.into_iter().map(ToString::to_string).collect(),
                    ..Default::default()
                };
                PandocBlock::CodeBlock(attr, code)
            }
            Block::MathBlock(code) => {
                PandocBlock::Para(vec![PandocInline::Math(MathType::DisplayMath, code)])
            }
            Block::Table(num_cols, head, body) => {
                let convert_row = |row: Row| {
                    let cells = row
                        .into_iter()
                        .map(|cell| PandocCell {
                            content: convert_blocks_to_pandoc(cell.blocks, anchors),
                            ..Default::default()
                        })
                        .collect();

                    PandocRow {
                        attr: Attr::default(),
                        cells,
                    }
                };
                let head = convert_row(head);
                let body = body.into_iter().map(convert_row).collect();

                PandocBlock::Table(Table {
                    colspecs: vec![ColSpec::default(); num_cols],
                    head: TableHead {
                        rows: vec![head],
                        ..Default::default()
                    },
                    bodies: vec![TableBody {
                        body,
                        ..Default::default()
                    }],
                    ..Default::default()
                })
            }
            Block::BulletList(entries) => {
                let entries = entries
                    .into_iter()
                    .map(|entry| convert_blocks_to_pandoc(entry.blocks, anchors))
                    .collect();

                PandocBlock::BulletList(entries)
            }
            Block::OrderedList(entries) => {
                let entries = entries
                    .into_iter()
                    .map(|entry| convert_blocks_to_pandoc(entry.blocks, anchors))
                    .collect();

                PandocBlock::OrderedList(Default::default(), entries)
            }
            Block::DefinitionList(entries) => {
                let entries = entries
                    .into_iter()
                    .map(|(segment, blocks)| {
                        let inlines = convert_inlines_to_pandoc(segment, anchors);

                        let blocks = convert_blocks_to_pandoc(blocks, anchors);

                        (inlines, vec![blocks])
                    })
                    .collect();

                PandocBlock::DefinitionList(entries)
            }
        }
    }
}

pub(crate) fn convert_inlines_to_pandoc(
    inlines: Vec<Inline>,
    anchors: &HashMap<&str, &str>,
) -> Vec<PandocInline> {
    inlines
        .into_iter()
        .map(|inline| inline.into_pandoc(&anchors))
        .collect()
}

pub(crate) fn convert_blocks_to_pandoc(
    blocks: Vec<Block>,
    anchors: &HashMap<&str, &str>,
) -> Vec<PandocBlock> {
    blocks
        .into_iter()
        .map(|block| block.into_pandoc(&anchors))
        .collect()
}
