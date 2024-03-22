use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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
pub struct InputUpdate(pub HashSet<String>);
