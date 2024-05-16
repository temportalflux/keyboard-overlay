use kdlize::AsKdl;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Switch {
	pub pos: (f32, f32),
	pub side: Option<Side>,
}

impl kdlize::FromKdl<()> for Switch {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let x = node.next_f64_req()? as f32;
		let y = node.next_f64_req()? as f32;
		let side = node.get_str_opt_t::<Side>("side")?;
		Ok(Self { pos: (x, y), side })
	}
}

impl AsKdl for Switch {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		node.entry(self.pos.0 as f64);
		node.entry(self.pos.1 as f64);
		if let Some(side) = self.side {
			node.entry(("side", side.to_string()));
		}
		node
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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

impl std::str::FromStr for Side {
	type Err = InvalidSide;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"left" => Ok(Self::Left),
			"right" => Ok(Self::Right),
			_ => Err(InvalidSide(s.to_owned())),
		}
	}
}

#[derive(thiserror::Error, Debug)]
#[error("Invalid Side {0}, expecting \"left\" or \"right\"")]
pub struct InvalidSide(String);
