use toddle::query::{SharedFontLoader, FontQuery, FontClass};
use toddle::tables::{CharMap, Header, HorizontalMetrics};

use super::*;
use crate::size::{Size, Size2D};

/// The context for text layouting.
///
/// See [`LayoutContext`] for details about the fields.
#[derive(Copy, Clone)]
pub struct TextContext<'a, 'p> {
    pub loader: &'a SharedFontLoader<'p>,
    pub style: &'a TextStyle,
}

impl<'a, 'p> TextContext<'a, 'p> {
    /// Create a text context from a generic layout context.
    pub fn from_layout_ctx(ctx: LayoutContext<'a, 'p>) -> TextContext<'a, 'p> {
        TextContext {
            loader: ctx.loader,
            style: ctx.style,
        }
    }
}

/// Layouts text into a box.
///
/// There is no complex layout involved. The text is simply laid out left-
/// to-right using the correct font for each character.
pub fn layout_text(text: &str, ctx: TextContext) -> LayoutResult<Layout> {
    TextLayouter::new(text, ctx).layout()
}

/// Layouts text into boxes.
struct TextLayouter<'a, 'p> {
    ctx: TextContext<'a, 'p>,
    text: &'a str,
    actions: LayoutActionList,
    buffer: String,
    active_font: usize,
    width: Size,
    classes: Vec<FontClass>,
}

impl<'a, 'p> TextLayouter<'a, 'p> {
    /// Create a new text layouter.
    fn new(text: &'a str, ctx: TextContext<'a, 'p>) -> TextLayouter<'a, 'p> {
        TextLayouter {
            ctx,
            text,
            actions: LayoutActionList::new(),
            buffer: String::new(),
            active_font: std::usize::MAX,
            width: Size::zero(),
            classes: ctx.style.classes.clone(),
        }
    }

    /// Layout the text
    fn layout(mut self) -> LayoutResult<Layout> {
        for c in self.text.chars() {
            let (index, char_width) = self.select_font(c)?;

            self.width += char_width;

            if self.active_font != index {
                if !self.buffer.is_empty() {
                    self.actions.add(LayoutAction::WriteText(self.buffer));
                    self.buffer = String::new();
                }

                self.actions.add(LayoutAction::SetFont(index, self.ctx.style.font_size));
                self.active_font = index;
            }

            self.buffer.push(c);
        }

        if !self.buffer.is_empty() {
            self.actions.add(LayoutAction::WriteText(self.buffer));
        }

        Ok(Layout {
            dimensions: Size2D::new(self.width, Size::pt(self.ctx.style.font_size)),
            actions: self.actions.into_vec(),
            debug_render: false,
        })
    }

    /// Select the best font for a character and return its index along with
    /// the width of the char in the font.
    fn select_font(&mut self, c: char) -> LayoutResult<(usize, Size)> {
        let mut loader = self.ctx.loader.borrow_mut();

        for class in &self.ctx.style.fallback {
            self.classes.push(class.clone());

            let query = FontQuery {
                chars: &[c],
                classes: &self.classes,
            };

            if let Some((font, index)) = loader.get(query) {
                let font_unit_ratio = 1.0 / (font.read_table::<Header>()?.units_per_em as f32);
                let font_unit_to_size = |x| Size::pt(font_unit_ratio * x);

                let glyph = font
                    .read_table::<CharMap>()?
                    .get(c)
                    .expect("layout text: font should have char");

                let glyph_width = font
                    .read_table::<HorizontalMetrics>()?
                    .get(glyph)
                    .expect("layout text: font should have glyph")
                    .advance_width as f32;

                let char_width = font_unit_to_size(glyph_width) * self.ctx.style.font_size;

                return Ok((index, char_width));
            }

            self.classes.pop();
        }

        Err(LayoutError::NoSuitableFont(c))
    }
}