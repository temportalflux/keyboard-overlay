use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SwitchLocation {
	pub pos: (f32, f32),
	pub side: Option<Side>,
}

impl kdlize::FromKdl<()> for SwitchLocation {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let x = node.next_f64_req()? as f32;
		let y = node.next_f64_req()? as f32;
		let side = node.get_str_opt_t::<Side>("side")?;
		Ok(Self { pos: (x, y), side })
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
