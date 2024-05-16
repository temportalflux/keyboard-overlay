use serde::{Deserialize, Serialize};

mod layout;
pub use layout::*;
mod key;
pub use key::*;
mod switch;
pub use switch::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogRecord {
	pub level: ::log::Level,
	pub target: String,
	pub file: Option<String>,
	pub line: Option<u32>,
	pub args: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InputUpdate {
	LayerActivate(String),
	LayerDeactivate(String),
	SwitchPressed(String, SwitchSlot),
	SwitchReleased(String),
}
