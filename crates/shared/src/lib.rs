use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogRecord {
	pub level: ::log::Level,
	pub target: String,
	pub file: Option<String>,
	pub line: Option<u32>,
	pub args: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layout(pub Vec<KeySwitch>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Side {
	Left,
	Right,
}
impl std::fmt::Display for Side {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Side::Left => "left",
				Side::Right => "right",
			}
		)
	}
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeySwitch {
	pub switch_id: String,

	pub pos: (f32, f32),
	pub side: Option<Side>,

	pub key_name: String,
	pub key_source: InputSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum InputSource {
	Keyboard,
	Mouse,
}
impl std::fmt::Display for InputSource {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::Keyboard => "keyboard",
				Self::Mouse => "mouse",
			}
		)
	}
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InputUpdate(pub HashSet<String>);
