use iced::{
    advanced::{image, layout, mouse, renderer, widget::Tree, Widget},
    widget, Element, Event, Length, Size,
};

use crate::{
    message::Message::{self},
    ppu::{LCD_HORIZONTAL_PIXEL_COUNT, LCD_VERTICAL_PIXEL_COUNT},
};

pub struct LCD<'a> {
    pub lcd_widget: Element<'a, Message>,
}

const LCD_ZOOM_FACTOR: usize = 5;

const ZOOMED_LCD_HORIZONTAL_PIXEL_COUNT: usize = LCD_HORIZONTAL_PIXEL_COUNT * LCD_ZOOM_FACTOR;
const ZOOMED_LCD_VERTICAL_PIXEL_COUNT: usize = LCD_VERTICAL_PIXEL_COUNT * LCD_ZOOM_FACTOR;

impl<'a> LCD<'a> {
    pub fn new(pixels: &[u8]) -> Self {
        LCD {
            lcd_widget: widget::Image::new(image::Handle::from_rgba(
                LCD_HORIZONTAL_PIXEL_COUNT as u32,
                LCD_VERTICAL_PIXEL_COUNT as u32,
                bytes::Bytes::copy_from_slice(pixels),
            ))
            .content_fit(iced::ContentFit::Fill)
            .filter_method(image::FilterMethod::Nearest)
            .width(ZOOMED_LCD_HORIZONTAL_PIXEL_COUNT as f32)
            .height(ZOOMED_LCD_VERTICAL_PIXEL_COUNT as f32)
            .into(),
        }
    }
}

impl<'a> Widget<Message, iced::Theme, iced::Renderer> for LCD<'a> {
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fixed(ZOOMED_LCD_HORIZONTAL_PIXEL_COUNT as f32),
            height: Length::Fixed(ZOOMED_LCD_VERTICAL_PIXEL_COUNT as f32),
        }
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.lcd_widget)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.lcd_widget]);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let lcd_layout =
            self.lcd_widget
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, limits);
        layout::Node::with_children(
            Size::new(
                ZOOMED_LCD_HORIZONTAL_PIXEL_COUNT as f32,
                ZOOMED_LCD_VERTICAL_PIXEL_COUNT as f32,
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
            self.lcd_widget.as_widget().draw(
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
                let pixel_x = (position.x - widget_bounds.x) as usize / LCD_ZOOM_FACTOR;
                let pixel_y = (position.y - widget_bounds.y) as usize / LCD_ZOOM_FACTOR;
                shell.publish(Message::MouseOnLCDPixel(pixel_x as u8, pixel_y as u8));
            }
        }
    }
}

impl<'a> From<LCD<'a>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(widget: LCD<'a>) -> Self {
        Element::new(widget)
    }
}
