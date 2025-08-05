use iced::widget::{self, row, Column};

use crate::message::Message;

pub fn view(pixel: (u8, u8)) -> Column<'static, Message> {
    let (x, y) = pixel;
    widget::Column::new().push(row![widget::text(format!("{}, {}", x, y))])
}
