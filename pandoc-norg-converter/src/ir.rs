use pandoc_types::definition::{
    Attr, Block as PandocBlock, Cell as PandocCell, ColSpec, Inline as PandocInline, MathType,
    Row as PandocRow, Table, TableBody, TableHead, Target,
};

use crate::document::{DocumentContext, DocumentLinkType};

#[derive(Debug, PartialEq, Eq)]
pub enum LinkType<'source> {
    None,
    Href(&'source str),
    File(&'source str),
    DocumentLink(DocumentLinkType, &'source str),
}

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

    Link(Vec<Inline<'source>>, LinkType<'source>),
    Anchor(Vec<Inline<'source>>, &'source str),

    Image(&'source str),
}

impl<'source> Inline<'source> {
    pub fn into_pandoc(self, context: &DocumentContext) -> PandocInline {
        match self {
            Inline::Space => PandocInline::Space,
            Inline::Str(str) => PandocInline::Str(str.to_string()),
            Inline::Emph(inlines) => {
                PandocInline::Emph(convert_inlines_to_pandoc(inlines, context))
            }
            Inline::Strong(inlines) => {
                PandocInline::Strong(convert_inlines_to_pandoc(inlines, context))
            }
            Inline::Underline(inlines) => {
                PandocInline::Underline(convert_inlines_to_pandoc(inlines, context))
            }
            Inline::Strikeout(inlines) => {
                PandocInline::Strikeout(convert_inlines_to_pandoc(inlines, context))
            }
            Inline::Subscript(inlines) => {
                PandocInline::Subscript(convert_inlines_to_pandoc(inlines, context))
            }
            Inline::Superscript(inlines) => {
                PandocInline::Superscript(convert_inlines_to_pandoc(inlines, context))
            }
            Inline::Code(str) => PandocInline::Code(Attr::default(), str.to_string()),
            Inline::Math(str) => PandocInline::Math(MathType::InlineMath, str.to_string()),
            Inline::Link(inlines, ty) => {
                let url = get_link_url(&ty, context);

                PandocInline::Link(
                    Attr::default(),
                    convert_inlines_to_pandoc(inlines, context),
                    Target {
                        url,
                        title: String::new(),
                    },
                )
            }
            Inline::Anchor(inlines, id) => {
                let url = context
                    .anchors
                    .get(id)
                    .map(|ty| get_link_url(ty, context))
                    .unwrap_or_default();

                PandocInline::Link(
                    Attr::default(),
                    convert_inlines_to_pandoc(inlines, context),
                    Target {
                        url,
                        title: String::new(),
                    },
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

fn get_link_url(ty: &LinkType, context: &DocumentContext) -> String {
    match *ty {
        LinkType::None => String::new(),
        LinkType::Href(url) => url.to_string(),
        LinkType::File(url) => url.to_string(),
        LinkType::DocumentLink(ref ty, text) => {
            let res = context.get_document_link(text, ty).cloned();

            if res.is_none() {
                log::warn!("Missing document link for {}", text);
            }

            res.unwrap_or_default()
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
    pub fn into_pandoc(self, context: &DocumentContext) -> PandocBlock {
        match self {
            Block::Null => PandocBlock::Null,
            Block::Plain(segment) => {
                let inlines = convert_inlines_to_pandoc(segment, context);

                PandocBlock::Plain(inlines)
            }
            Block::Paragraph(segments) => {
                let mut inlines = Vec::new();
                let mut segments = segments.into_iter();

                if let Some(segment) = segments.next() {
                    inlines.extend(convert_inlines_to_pandoc(segment, context));
                }

                for segment in segments {
                    inlines.push(PandocInline::Space);
                    inlines.extend(convert_inlines_to_pandoc(segment, context));
                }

                PandocBlock::Para(inlines)
            }
            Block::Header(level, attr, segment) => {
                let inlines = convert_inlines_to_pandoc(segment, context);

                PandocBlock::Header(level, attr, inlines)
            }
            Block::BlockQuote(blocks) => {
                let blocks = convert_blocks_to_pandoc(blocks, context);
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
                            content: convert_blocks_to_pandoc(cell.blocks, context),
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
                    .map(|entry| convert_blocks_to_pandoc(entry.blocks, context))
                    .collect();

                PandocBlock::BulletList(entries)
            }
            Block::OrderedList(entries) => {
                let entries = entries
                    .into_iter()
                    .map(|entry| convert_blocks_to_pandoc(entry.blocks, context))
                    .collect();

                PandocBlock::OrderedList(Default::default(), entries)
            }
            Block::DefinitionList(entries) => {
                let entries = entries
                    .into_iter()
                    .map(|(segment, blocks)| {
                        let inlines = convert_inlines_to_pandoc(segment, context);

                        let blocks = convert_blocks_to_pandoc(blocks, context);

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
    context: &DocumentContext,
) -> Vec<PandocInline> {
    inlines
        .into_iter()
        .map(|inline| inline.into_pandoc(context))
        .collect()
}

pub(crate) fn convert_blocks_to_pandoc(
    blocks: Vec<Block>,
    context: &DocumentContext,
) -> Vec<PandocBlock> {
    blocks
        .into_iter()
        .map(|block| block.into_pandoc(context))
        .collect()
}
