use iced::{Align, Application, button, Button, Column, Command, Container, Element, executor, Length, Settings, Subscription, Text, Row, window, Scrollable, scrollable, Color, TextInput, text_input};
use crate::damuku::DanmakuClient;
use futures::channel::mpsc::{unbounded, UnboundedReceiver};
use crate::entity::LiveMsg;
use async_std::sync::{Mutex, Arc};
use crate::textsink::sink;
use font_kit::source::SystemSource;
use once_cell::sync::OnceCell;

pub mod common;
pub mod damuku;
pub mod entity;
pub mod textsink;
pub mod timer;

fn global_font() -> &'static [u8] {
    static INSTANCE: OnceCell<Arc<Vec<u8>>> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        SystemSource::new()
            .select_family_by_name("Microsoft YaHei UI")
            .unwrap()
            .fonts()
            .last()
            .unwrap()
            .load()
            .unwrap()
            .copy_font_data()
            .unwrap()
    })
}

pub fn start() -> iced::Result {
    common::init_logger();

    let setting = Settings {
        default_font: Some(global_font()),
        window: window::Settings {
            size: (400, 500),
            min_size: Some((100, 100)),
            max_size: None,
            resizable: true,
            decorations: true,
            transparent: true,
            always_on_top: false,
            icon: None,
        },
        ..Default::default()
    };

    Example::run(setting)
}

#[derive(Debug)]
enum Example {
    Idle {
        button_state: button::State,
        text_input_state: text_input::State,
        room_id: String,
    },
    Running {
        room_id: String,
        lines: Vec<LiveMsg>,
        button_state: button::State,
        scroll_state: scrollable::State,
        client: DanmakuClient,
        live_msg_rx: Arc<Mutex<UnboundedReceiver<LiveMsg>>>,
    },

}

#[derive(Debug, Clone)]
pub enum Message {
    Start,
    LiveMsg(Option<LiveMsg>),
    Stop,
    InputRoomId(String),
}

impl Application for Example {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Example, Command<Message>) {
        (
            Example::Idle {
                button_state: Default::default(),
                text_input_state: Default::default(),
                room_id: "".to_string(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Danmuku Machine")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Start => {
                log::info!("[update] message=start");
                let (tx, rx) = unbounded();

                if let Example::Idle { room_id, .. } = self {
                    if !room_id.is_empty() {
                        *self = Example::Running {
                            room_id: room_id.to_owned(),
                            lines: vec![],
                            button_state: Default::default(),
                            scroll_state: Default::default(),
                            client: DanmakuClient::new(room_id.to_owned(), tx),
                            live_msg_rx: Arc::new(Mutex::new(rx)),
                        };
                    }
                };
            }
            Message::LiveMsg(text_option) => {
                if let Some(text) = text_option {
                    log::debug!("[update] message=LiveMsg, cmd={}", &text.cmd);
                    if let Example::Running { lines, .. } = self {
                        lines.push(text);
                        while lines.len() > 100 {
                            lines.pop();
                        }
                    }
                }
            }
            Message::Stop => {
                log::info!("[update] message=stop");
                *self = Example::Idle {
                    button_state: Default::default(),
                    text_input_state: Default::default(),
                    room_id: "".to_string(),
                };
            }
            Message::InputRoomId(text) => {
                match self {
                    Example::Idle { room_id, .. } => *room_id = text,
                    Example::Running { room_id, .. } => *room_id = text,
                }
            }
        };

        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        match self {
            Example::Idle { .. } => { Subscription::none() }
            Example::Running { live_msg_rx, room_id, .. } => {
                sink(room_id.to_owned(), live_msg_rx.clone()).map(Message::LiveMsg)
            }
        }
    }

    fn background_color(&self) -> Color {
        Color::new(0.3, 0.3, 0.3, 0.3)
    }

    fn view(&mut self) -> Element<Message> {
        let content = match self {
            Example::Idle {
                button_state,
                text_input_state,
                room_id,
            } => {
                let button = Button::new(button_state, Text::new("Start"))
                    .on_press(Message::Start);
                let text_input = TextInput::new(text_input_state, "Room ID", room_id, Message::InputRoomId);

                Row::new()
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_items(Align::Start)
                    .push(
                        Column::new()
                            .align_items(Align::Start)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .push(Row::new()
                                .width(Length::Fill)
                                .height(Length::Units(32))
                                .push(button)
                                .push(text_input.padding(10))
                            )
                    )
            }
            Example::Running {
                lines,
                button_state,
                scroll_state,
                room_id,
                ..
            } => {
                let button = Button::new(button_state, Text::new("Stop"))
                    .width(Length::Units(60))
                    .on_press(Message::Stop);

                let mut scrollable = Scrollable::new(scroll_state).height(Length::Fill);
                for line in lines {
                    if let Some(t) = line.as_iced_text() {
                        scrollable = scrollable.push(t);
                    }
                }


                Row::new()
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_items(Align::Start)
                    .push(
                        Column::new()
                            .align_items(Align::Start)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .push(
                                Row::new()
                                    .width(Length::Fill)
                                    .push(button)
                                    .push(
                                        Column::new().push(
                                            Text::new(format!("Room ID: {}", room_id))
                                                .width(Length::Fill)
                                        ).width(Length::Fill).padding(5)
                                    )
                            )
                            .push(scrollable)
                    )
            }
        };

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}
