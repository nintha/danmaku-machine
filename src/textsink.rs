use async_std::sync::{Arc, Mutex};
use iced::{futures, Recipe};
use iced::futures::channel::mpsc::UnboundedReceiver;
use iced::futures::stream::BoxStream;
use iced::futures::StreamExt;

use crate::entity::LiveMsg;

pub fn sink(room_id: String, rx: Arc<Mutex<UnboundedReceiver<LiveMsg>>>) -> iced::Subscription<Option<LiveMsg>> {
    iced::Subscription::from_recipe(Download {
        room_id,
        rx,
    })
}


pub struct Download {
    room_id: String,
    rx: Arc<Mutex<UnboundedReceiver<LiveMsg>>>,
}

impl<H, I> Recipe<H, I> for Download
    where
        H: std::hash::Hasher,
{
    type Output = Option<LiveMsg>;

    fn hash(&self, state: &mut H) {
        use std::hash::Hash;

        std::any::TypeId::of::<Self>().hash(state);
        self.room_id.hash(state);
    }

    fn stream(self: Box<Self>, _input: BoxStream<'static, I>) -> BoxStream<'static, Self::Output> {
        Box::pin(futures::stream::unfold(State::Running(self.rx.clone()), |state| async move {
            match state {
                State::Running(rx) => {
                    let mut rx_guard = rx.lock().await;
                    if let Some(live_msg) = rx_guard.next().await {
                        Some((Some(live_msg), State::Running(rx.clone())))
                    } else {
                        Some((None, State::Finished))
                    }
                }
                State::Finished => { None }
            }
        }))
    }
}

pub enum State {
    Running(Arc<Mutex<UnboundedReceiver<LiveMsg>>>),
    Finished,
}

