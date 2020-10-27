use std::convert::TryInto;
use std::io::Read;
use std::time::Duration;

use async_tungstenite::async_std::connect_async;
use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures::{SinkExt, StreamExt, FutureExt};
use serde_json::Value;

use crate::entity::LiveMsg;
use futures::channel::oneshot;
use crate::timer::Timer;

#[derive(Debug)]
pub struct DanmakuClient {
    room_id: String,
    _stop_sender: oneshot::Sender<()>,
}

impl DanmakuClient {
    const SERVER_URL: &'static str = "wss://broadcastlv.chat.bilibili.com:2245/sub";

    pub fn new(room_id: String, sender: UnboundedSender<LiveMsg>) -> Self {
        let (stop_tx, stop_rx) = oneshot::channel();

        let room_id_clone = room_id.clone();
        async_std::task::spawn(async move{
            Self::run_loop(room_id_clone, stop_rx, sender).await.unwrap()
        });

        Self{
            room_id,
            _stop_sender: stop_tx
        }
    }

    async fn run_loop(room_id: String,  stop_emitter: oneshot::Receiver<()>, sender: UnboundedSender<LiveMsg>) -> anyhow::Result<()> {
        let ws_stream = connect_async(Self::SERVER_URL).await.expect("Failed to connect").0;
        log::info!("WebSocket handshake has been successfully completed");
        let (mut ws_sender, mut ws_reader) = ws_stream.split();
        let (ch_tx, mut ch_rx) = unbounded::<Option<Vec<u8>>>();

        async_std::task::spawn(async move {
            while let Some(Some(bytes)) = ch_rx.next().await {
                log::debug!("[send] {:?}", &bytes);
                if let Err(e) = ws_sender.send(bytes.into()).await {
                    log::error!("send message error, {:?}", e);
                }
            }
            log::info!("sender handle closed");
        });

        Self::shake_hand(&room_id,ch_tx.clone())?;
        let _timer = Self::start_heartbeat_timer(ch_tx.clone());


        let mut stop_emitter = stop_emitter.fuse();
        loop {

            let mut reader_future = ws_reader.next().fuse();
            futures::select! {
                reader_result = reader_future => {
                    if let Some(Ok(message)) = reader_result{
                        let bytes = message.into_data();
                        if let Ok(msgs) = parse_message(&bytes) {
                            for msg in msgs {
                                if let Err(e) = sender.unbounded_send(msg) {
                                    log::info!("[sender] error, {:?}", e);
                                    break;
                                }
                            }
                        }
                    }else{
                        break;
                    }
                }
                option = stop_emitter => {
                    log::info!("[stop_emitter] emit, {:?}", option);
                    break;
                }
            }
        }
        // close the channel
        ch_tx.unbounded_send(None)?;
        log::info!("[DanmakuClient] loop over");
        Ok(())
    }

    fn shake_hand(room_id: &str, tx: UnboundedSender<Option<Vec<u8>>>) -> anyhow::Result<()> {
        tx.unbounded_send(Some(build_init_msg(&room_id)))?;
        Ok(())
    }


    fn start_heartbeat_timer(tx: UnboundedSender<Option<Vec<u8>>>) -> Timer {
        let message: [u8; 16] = hex_literal::hex!("00000010001000010000000200000001");
        Timer::new(|(tx, message)| async move {
            if let Err(e) = tx.unbounded_send(Some(message.to_vec())) {
                log::error!("send to channel error, {:?}", e);
            }
        }, (tx, message), Duration::from_secs(30), )
    }
}

fn build_init_msg(room_id: &str) -> Vec<u8> {
    let mut sub_header: Vec<u8> = hex_literal::hex!("001000010000000700000001").to_vec();
    let body = format!("{{\"roomid\":{}}}", room_id);
    let len = (body.len() + 16) as u32;
    let mut message = len.to_be_bytes().to_vec();
    message.append(&mut sub_header);
    message.append(&mut body.as_bytes().to_vec());
    message
}


fn parse_message(bytes: &[u8]) -> anyhow::Result<Vec<LiveMsg>> {
    let vec = spilt_message(bytes)?;
    let mut messages = vec![];
    for frame in vec {
        let mut msg = parse_frame(frame)?;
        messages.append(&mut msg);
    }
    Ok(messages)
}

/// 分离websocket消息，消息中包含多个数据帧
fn spilt_message(bytes: &[u8]) -> anyhow::Result<Vec<&[u8]>> {
    let mut frames: Vec<&[u8]> = vec![];

    let total_len = bytes.len();
    let mut offset_start;
    let mut offset_end = 0usize;

    loop {
        if offset_end >= total_len {
            break;
        }
        offset_start = offset_end;

        let len_bytes = &bytes[offset_start..offset_start + 4];
        let len = u32::from_be_bytes(len_bytes.try_into()?) as usize;
        if len < 16 {
            // invalid frame length
            break;
        }

        offset_end = offset_start + len;
        let frame = &bytes[offset_start..offset_end];
        frames.push(frame);
    }

    Ok(frames)
}

/// 解析数据帧
fn parse_frame(frame: &[u8]) -> anyhow::Result<Vec<LiveMsg>> {
    let ver = u16::from_be_bytes((&frame[6..8]).try_into()?);
    let action = u32::from_be_bytes((&frame[8..12]).try_into()?);
    let payload = &frame[16..];

    log::debug!("ver={}, action={}, payload_len={}", ver, action, payload.len());
    let messages = match ver {
        0 => {
            if action == 5 {
                let v: Value = serde_json::from_slice(payload)?;
                log::info!("[json] {}", v.to_string());
                let live_msg: LiveMsg = serde_json::from_value(v)?;
                vec![live_msg]
            } else {
                log::warn!("unknown action {}", action);
                vec![]
            }
        }
        // 人气值，不做处理
        1 => vec![],
        // 压缩封装了多个message
        2 => {
            let mut text = vec![];
            libflate::non_blocking::zlib::Decoder::new(payload).read_to_end(&mut text)?;
            parse_message(&text)?
        }
        _ => {
            log::warn!("unknown ver {}", ver);
            vec![]
        }
    };

    Ok(messages)
}
