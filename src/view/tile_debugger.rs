use iced::widget::{self, row, Column};

use crate::message::Message;

pub fn view(tile_id: u8) -> Column<'static, Message> {
    widget::Column::new().push(row![widget::text(format!("Looking at tile {}", tile_id))])
}
