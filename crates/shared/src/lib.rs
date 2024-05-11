use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InputState {
	pub active_layers: HashSet<String>,
	pub active_switches: BTreeMap<String, SwitchSlot>,
}

impl InputState {
	pub fn is_layer_active(&self, layer_id: &String) -> bool {
		self.active_layers.contains(layer_id)
	}
}
