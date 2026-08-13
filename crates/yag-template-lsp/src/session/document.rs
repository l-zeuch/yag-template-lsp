use rowan::{TextRange, TextSize};
use tower_lsp_server::ls_types::{Location, Position, Range, Uri};
use yag_template_analysis::Analysis;
use yag_template_envdefs::EnvDefs;
use yag_template_syntax::ast::ext::SyntaxNodeExt;
use yag_template_syntax::parser::Parse;
use yag_template_syntax::query::Query;
use yag_template_syntax::{SyntaxNode, ast, parser};

use super::Session;

pub(crate) struct Document {
    pub(crate) uri: Uri,
    pub(crate) parse: Parse,
    pub(crate) mapper: Mapper,
    pub(crate) analysis: Analysis,
}

impl Document {
    pub(crate) fn new(sess: &Session, uri: Uri, src: &str) -> Self {
        let envdefs = sess.read_envdefs();
        let parse = parser::parse(src);
        let root = SyntaxNode::new_root(parse.root.clone()).to::<ast::Root>();
        Self {
            uri,
            parse,
            mapper: Mapper::new(src),
            analysis: yag_template_analysis::analyze(&envdefs, root),
        }
    }

    pub(crate) fn source(&self) -> &str {
        &self.mapper.text
    }

    pub(crate) fn reanalyze_with(&self, envdefs: &EnvDefs) -> Self {
        let root = SyntaxNode::new_root(self.parse.root.clone()).to::<ast::Root>();
        Self {
            uri: self.uri.clone(),
            parse: self.parse.clone(),
            mapper: self.mapper.clone(),
            analysis: yag_template_analysis::analyze(envdefs, root),
        }
    }

    pub(crate) fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.parse.root.clone())
    }

    pub(crate) fn query_at(&self, pos: Position) -> Query {
        Query::at(&self.syntax(), self.mapper.offset(pos))
    }

    pub(crate) fn location_for(&self, range: TextRange) -> Location {
        Location::new(self.uri.clone(), self.mapper.range(range))
    }
}

/// A mapper that translates between byte offsets and 0-based line:UTF-16-character positions.
#[derive(Clone)]
pub(crate) struct Mapper {
    text: String,
    line_starts: Vec<TextSize>, // 0, plus byte offsets immediately preceding newlines
}

impl Mapper {
    pub(crate) fn new(text: &str) -> Self {
        let mut line_starts = vec![TextSize::from(0)];
        line_starts.extend(
            text.match_indices('\n')
                .map(|(lf_offset, _)| TextSize::from(lf_offset as u32 + 1)),
        );
        Self {
            text: text.to_owned(),
            line_starts,
        }
    }

    pub(crate) fn offset(&self, position: Position) -> TextSize {
        let Some(&line_start) = self.line_starts.get(position.line as usize) else {
            return TextSize::from(self.text.len() as u32);
        };
        line_start + TextSize::from(utf16_col_to_byte(self.line_text(position.line), position.character))
    }

    pub(crate) fn text_range(&self, range: Range) -> TextRange {
        TextRange::new(self.offset(range.start), self.offset(range.end))
    }

    /// Panics if `offset` is past the end of the text or falls inside a character.
    pub(crate) fn position(&self, offset: TextSize) -> Position {
        let (line, line_start) = self.line_at(offset);
        let character = utf16_len(&self.text[usize::from(line_start)..usize::from(offset)]);
        Position { line, character }
    }

    pub(crate) fn range(&self, range: TextRange) -> Range {
        Range {
            start: self.position(range.start()),
            end: self.position(range.end()),
        }
    }

    /// The line that `offset` falls on, along with that line's start offset.
    fn line_at(&self, offset: TextSize) -> (u32, TextSize) {
        let line = self.line_starts.partition_point(|&line_start| line_start <= offset) - 1;
        (line as u32, self.line_starts[line])
    }

    /// The text of `line`, excluding its trailing line feed. A carriage return in a
    /// CRLF sequence is retained, since it occupies a character position of its own.
    fn line_text(&self, line: u32) -> &str {
        let start = usize::from(self.line_starts[line as usize]);
        let end = match self.line_starts.get(line as usize + 1) {
            Some(&next_line_start) => usize::from(next_line_start) - 1,
            None => self.text.len(),
        };
        &self.text[start..end]
    }
}

fn utf16_len(text: &str) -> u32 {
    if text.is_ascii() {
        text.len() as u32
    } else {
        text.chars().map(|c| c.len_utf16() as u32).sum()
    }
}

/// The byte offset within `line` of the given 0-based UTF-16 character offset,
/// clamped to the length of the line.
fn utf16_col_to_byte(line: &str, character: u32) -> u32 {
    if line.is_ascii() {
        return character.min(line.len() as u32);
    }

    let mut utf16_col = 0;
    for (byte_offset, c) in line.char_indices() {
        if utf16_col >= character {
            return byte_offset as u32;
        }
        utf16_col += c.len_utf16() as u32;
    }
    line.len() as u32
}
