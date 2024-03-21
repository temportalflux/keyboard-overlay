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
pub struct Layout;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InputUpdate;
