use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyBinding {
	pub key: String,
	pub source: InputSource,
}

impl Default for KeyBinding {
	fn default() -> Self {
		Self { key: "empty".into(), source: InputSource::Keyboard }
	}
}

impl kdlize::FromKdl<()> for KeyBinding {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let source = node.next_str_req_t()?;
		let key = node.next_str_req()?.to_owned();
		Ok(Self { key, source })
	}
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

impl std::str::FromStr for InputSource {
	type Err = InvalidInputSource;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"keyboard" => Ok(Self::Keyboard),
			"mouse" => Ok(Self::Mouse),
			_ => Err(InvalidInputSource(s.to_owned())),
		}
	}
}

#[derive(thiserror::Error, Debug)]
#[error("Invalid InputSource {0}, expecting \"keyboard\" or \"mouse\"")]
pub struct InvalidInputSource(String);
