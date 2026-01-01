use iced::{
    advanced::{image, layout, mouse, renderer, widget::Tree, Widget},
    widget, Element, Event, Length, Size,
};

use crate::{
    message::Message::{self},
    ppu::{
        self, HORIZONTAL_PIXELS_PER_TILE, TILE_PALETTE_HORIZONTAL_PIXELS,
        TILE_PALETTE_HORIZONTAL_TILE_COUNT, TILE_PALETTE_VERTICAL_PIXELS, VERTICAL_PIXELS_PER_TILE,
    },
};

pub struct TilePalette<'a> {
    pub tile_palette_widget: Element<'a, Message>,
}

const TILE_PALETTE_ZOOM_FACTOR: usize = 4;
const ZOOMED_TILE_PALETTE_HORIZONTAL_PIXEL_COUNT: usize =
    TILE_PALETTE_HORIZONTAL_PIXELS * TILE_PALETTE_ZOOM_FACTOR;
const ZOOMED_TILE_PALETTE_VERTICAL_PIXEL_COUNT: usize =
    TILE_PALETTE_VERTICAL_PIXELS * TILE_PALETTE_ZOOM_FACTOR;

impl<'a> TilePalette<'a> {
    pub fn new(tiles_pixels: &ppu::TilePalettePixels) -> Self {
        TilePalette {
            tile_palette_widget: widget::Image::new(image::Handle::from_rgba(
                TILE_PALETTE_HORIZONTAL_PIXELS as u32,
                TILE_PALETTE_VERTICAL_PIXELS as u32,
                bytes::Bytes::copy_from_slice(&tiles_pixels.pixels),
            ))
            .content_fit(iced::ContentFit::Fill)
            .filter_method(image::FilterMethod::Nearest)
            .width(ZOOMED_TILE_PALETTE_HORIZONTAL_PIXEL_COUNT as f32)
            .height(ZOOMED_TILE_PALETTE_VERTICAL_PIXEL_COUNT as f32)
            .into(),
        }
    }
}

impl<'a> Widget<Message, iced::Theme, iced::Renderer> for TilePalette<'a> {
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fixed(ZOOMED_TILE_PALETTE_HORIZONTAL_PIXEL_COUNT as f32),
            height: Length::Fixed(ZOOMED_TILE_PALETTE_VERTICAL_PIXEL_COUNT as f32),
        }
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.tile_palette_widget)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.tile_palette_widget]);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let lcd_layout = self.tile_palette_widget.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            limits,
        );
        layout::Node::with_children(
            Size::new(
                ZOOMED_TILE_PALETTE_HORIZONTAL_PIXEL_COUNT as f32,
                ZOOMED_TILE_PALETTE_VERTICAL_PIXEL_COUNT as f32,
            ),
            vec![lcd_layout],
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        let mut children = layout.children();

        if let Some(lcd_layout) = children.next() {
            self.tile_palette_widget.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                lcd_layout,
                cursor,
                viewport,
            );
        }
    }

    fn update(
        &mut self,
        _state: &mut Tree,
        event: &iced::Event,
        layout: layout::Layout<'_>,
        _cursor: iced::advanced::mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        _viewport: &iced::Rectangle,
    ) {
        if let Event::Mouse(mouse::Event::CursorMoved { position }) = event {
            let widget_bounds = layout.bounds();
            if position.x >= widget_bounds.x
                && position.x <= widget_bounds.x + widget_bounds.width
                && position.y >= widget_bounds.y
                && position.y <= widget_bounds.y + widget_bounds.height
            {
                let tile_x = (position.x - widget_bounds.x) as usize
                    / (HORIZONTAL_PIXELS_PER_TILE * TILE_PALETTE_ZOOM_FACTOR);
                let tile_y = (position.y - widget_bounds.y) as usize
                    / (VERTICAL_PIXELS_PER_TILE * TILE_PALETTE_ZOOM_FACTOR);
                let tile_id = tile_y * TILE_PALETTE_HORIZONTAL_TILE_COUNT + tile_x;
                shell.publish(Message::MouseOnTilePalette(tile_id as u16));
            }
        }
    }
}

impl<'a> From<TilePalette<'a>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(widget: TilePalette<'a>) -> Self {
        Element::new(widget)
    }
}
