use serde::{Deserialize, Serialize};

mod binding;
pub use binding::*;
mod combo;
pub use combo::*;
mod key;
pub use key::*;
mod layer;
pub use layer::*;
mod layout;
pub use layout::*;
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
	SwitchPressed(String, Option<SwitchSlot>),
	SwitchReleased(String),
}
