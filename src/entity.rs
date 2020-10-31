use serde::Deserialize;
use serde_json::Value;
use iced::{Text, Color, Length};
use crate::DANMAKU_LINE_HEIGHT;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Data {
    #[serde(default)]
    pub uname: String,
    #[serde(default)]
    pub action: String,
    #[serde(default, rename = "giftName")]
    pub gift_name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LiveMsg {
    #[serde(default)]
    pub cmd: String,
    #[serde(default)]
    pub data: Data,
    #[serde(default)]
    pub info: Vec<Value>,
}

impl LiveMsg {
    pub const CMD_DANMU_MSG: &'static str = "DANMU_MSG";
    pub const CMD_SEND_GIFT: &'static str = "SEND_GIFT";

    pub fn is_danmaku(&self) -> bool {
        self.cmd == Self::CMD_DANMU_MSG
    }

    pub fn is_send_gift(&self) -> bool {
        self.cmd == Self::CMD_SEND_GIFT
    }

    /// 解析`cmd=DANMU_MSG`时的数据
    pub fn as_danmaku(&self) -> Option<Danmaku> {
        if self.cmd != Self::CMD_DANMU_MSG {
            return None;
        }

        let text = self.info.get(1).map(Value::as_str).flatten().unwrap_or_default().to_owned();
        let uname = self.info.get(2)
            .map(Value::as_array).flatten()
            .map(|x| x.get(1)).flatten()
            .map(Value::as_str).flatten()
            .unwrap_or_default().to_owned();

        Some(Danmaku { uname, text })
    }

    pub fn as_iced_text(&self) -> Option<Text> {
        match self.cmd.as_str() {
            Self::CMD_DANMU_MSG => {
                let danmaku = self.as_danmaku().unwrap();
                let text = format!("{}: {}", &danmaku.uname, &danmaku.text);
                Some(Text::new(text).height(Length::Units(DANMAKU_LINE_HEIGHT)))
            }
            Self::CMD_SEND_GIFT => {
                let text = format!("{}: {} {}\n", &self.data.uname, &self.data.action, &self.data.gift_name);
                Some(Text::new(text).color(Color::from_rgb(1.0, 0.5, 0.5)).height(Length::Units(DANMAKU_LINE_HEIGHT)))
            }
            _ => None
        }
    }
}


#[derive(Debug)]
pub struct Danmaku {
    pub uname: String,
    pub text: String,
}