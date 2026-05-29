use crate::prelude::*;

use anylm::Message;
use ratatui::layout::Rect;

/// The app state
pub struct AppState {
    pub tx: UnboundedSender<String>,

    pub input_area: Rect,
    pub input: String,
    pub input_cursor: usize,
    pub input_scroll: u16,

    pub chat_area: Rect,
    pub messages: Arc<Mutex<Vec<Message>>>,
    pub response_index: usize,
    pub chat_scroll: u16,

    pub commands: Vec<(&'static str, &'static str)>,
    pub status: Option<String>,

    pub tick_count: u64,
    pub is_busy: bool,
}

impl AppState {
    /// Creates a new app state
    pub fn new(tx: UnboundedSender<String>) -> Self {
        let commands = vec![
            ("/compact", "Compress the dialog context"),
            ("/clear", "Clear the dialog context"),
            ("/exit", "Exit the assistant"),
            ("/cancel", "Cancel the answer generation"),
        ];

        Self {
            tx,

            input_area: Default::default(),
            input: str!(),
            input_cursor: 0,
            input_scroll: 0,

            chat_area: Default::default(),
            messages: arc_mutex!(vec![]),
            response_index: 0,
            chat_scroll: 0,

            commands,
            status: None,

            tick_count: 0,
            is_busy: false,
        }
    }
}
