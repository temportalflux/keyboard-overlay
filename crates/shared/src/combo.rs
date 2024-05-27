use crate::{BindingDisplay, KeySet};
use kdlize::{
	ext::{EntryExt, ValueExt},
	AsKdl, FromKdl, OmitIfEmpty,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Combo {
	pub id: String,
	pub layers: Vec<String>,
	pub pos: (f32, f32),
	pub label: BindingDisplay,
	pub links: Vec<Link>,
	pub input: KeySet,
	pub input_layer: Option<String>,
}

impl FromKdl<()> for Combo {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let id = node.next_str_req()?.to_owned();
		let pos_x = node.next_f64_req()? as f32;
		let pos_y = node.next_f64_req()? as f32;
		let label = BindingDisplay::try_from(node.next_req()?)?;

		let mut layers = Vec::new();
		for mut node in node.query_all("scope() > layers")? {
			while let Some(entry) = node.next_opt() {
				layers.push(entry.as_str_req()?.to_owned());
			}
		}

		let links = node.query_all_t("scope() > link")?;

		let (input, input_layer) = {
			let mut node = node.query_req("scope() > bind")?;
			let input = node.next_str_req_t::<KeySet>()?;
			let layer = node.get_str_opt("layer")?.map(str::to_owned);
			(input, layer)
		};

		Ok(Self {
			id,
			layers,
			pos: (pos_x, pos_y),
			label,
			links,
			input,
			input_layer,
		})
	}
}

impl AsKdl for Combo {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		node.entry(self.id.as_str());
		node.entry(self.pos.0 as f64);
		node.entry(self.pos.1 as f64);
		node += self.label.as_kdl();
		node.child((
			{
				let mut node = kdlize::NodeBuilder::default();
				for layer in &self.layers {
					node.entry(layer.as_str());
				}
				node.build("layers")
			},
			OmitIfEmpty,
		));
		node.children(("link", &self.links));
		node.child(("bind", {
			let mut node = kdlize::NodeBuilder::default();
			node.entry(self.input.to_string());
			node.entry(("layer", self.input_layer.clone()));
			node
		}));
		node
	}
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Link(Vec<LinkPoint>);

impl Link {
	pub fn points(&self) -> &Vec<LinkPoint> {
		&self.0
	}
}

impl FromKdl<()> for Link {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let children = node.children().unwrap_or_default();
		let mut points = Vec::with_capacity(children.len());
		for mut node in children {
			points.push(LinkPoint::from_kdl(&mut node)?);
		}
		Ok(Self(points))
	}
}

impl AsKdl for Link {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		for point in &self.0 {
			node.child((point.node_id(), point.as_kdl()));
		}
		node
	}
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LinkPoint {
	Switch(String, f64, f64),
	Point {
		pos: (f64, f64),
		// -1 or +1 for the scalar of the control points on each axis
		control_dirs: (f64, f64),
		// the unsigned scalar for the distance from pos on each axis, signed by control dirs
		control_size: f64,
		// 0 for x, 1 for y; this drives which axis is offset for the incoming control point, and the other is the outgoing control point
		control_incoming_axis: u8,
	},
	Anchor(f64, f64),
}

impl LinkPoint {
	fn node_id(&self) -> &'static str {
		match self {
			Self::Switch(..) => "switch",
			Self::Point { .. } => "point",
			Self::Anchor(..) => "anchor",
		}
	}
}

#[derive(thiserror::Error, Debug)]
#[error("Invalid link point node id {0}, expecting \"switch\", \"point\", or \"anchor\"")]
pub struct InvalidLinkPointType(String);

#[derive(thiserror::Error, Debug)]
#[error("Invalid link point direction type {0}, expecting \"+\" or \"-\"")]
pub struct InvalidLinkPointDirection(String);

#[derive(thiserror::Error, Debug)]
#[error("Invalid link point direction type {0}, expecting \"X\" or \"Y\"")]
pub struct InvalidLinkPointAxis(String);

impl FromKdl<()> for LinkPoint {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		match node.name().value() {
			"switch" => {
				let switch_id = node.next_str_req()?.to_owned();
				let rel_x = node.next_f64_req()?;
				let rel_y = node.next_f64_req()?;
				Ok(Self::Switch(switch_id, rel_x, rel_y))
			}
			"point" => {
				let (control_dir_x, pos_x) = {
					let entry = node.next_req()?;
					let dir = match entry.type_req()? {
						"+" => 1f64,
						"-" => -1f64,
						ty => Err(InvalidLinkPointDirection(ty.to_owned()))?,
					};
					let pos = entry.as_f64_req()?;
					(dir, pos)
				};
				let (control_dir_y, pos_y) = {
					let entry = node.next_req()?;
					let dir = match entry.type_req()? {
						"+" => 1f64,
						"-" => -1f64,
						ty => Err(InvalidLinkPointDirection(ty.to_owned()))?,
					};
					let pos = entry.as_f64_req()?;
					(dir, pos)
				};
				let (control_incoming_axis, control_size) = {
					let entry = node.next_req()?;
					let dir = match entry.type_req()? {
						"X" => 0u8,
						"Y" => 1u8,
						ty => Err(InvalidLinkPointAxis(ty.to_owned()))?,
					};
					let size = entry.as_f64_req()?;
					(dir, size)
				};
				Ok(Self::Point {
					pos: (pos_x, pos_y),
					control_dirs: (control_dir_x, control_dir_y),
					control_size,
					control_incoming_axis,
				})
			}
			"anchor" => {
				let rel_x = node.next_f64_req()?;
				let rel_y = node.next_f64_req()?;
				Ok(Self::Anchor(rel_x, rel_y))
			}
			name => Err(InvalidLinkPointType(name.to_owned()))?,
		}
	}
}

impl AsKdl for LinkPoint {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		match self {
			Self::Switch(switch_id, rel_x, rel_y) => {
				node.entry(switch_id.as_str());
				node.entry(*rel_x);
				node.entry(*rel_y);
			}
			Self::Point {
				pos,
				control_dirs,
				control_incoming_axis,
				control_size,
			} => {
				node.entry_typed(if control_dirs.0 > 0.0 { "+" } else { "-" }, pos.0);
				node.entry_typed(if control_dirs.1 > 0.0 { "+" } else { "-" }, pos.1);
				node.entry_typed(if *control_incoming_axis == 0u8 { "X" } else { "Y" }, *control_size);
			}
			Self::Anchor(rel_x, rel_y) => {
				node.entry(*rel_x);
				node.entry(*rel_y);
			}
		}
		node
	}
}
