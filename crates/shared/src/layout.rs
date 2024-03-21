use crate::{KeyBinding, SwitchLocation};
use kdlize::{ext::DocumentExt, FromKdl};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layout {
	switches: HashMap<String, SwitchLocation>,
	default_layer: String,
	layers: HashMap<String, Layer>,
}

impl Layout {
	pub fn default_layer(&self) -> &String {
		&self.default_layer
	}

	pub fn switches(&self) -> &HashMap<String, SwitchLocation> {
		&self.switches
	}

	pub fn get_layer(&self, id: impl AsRef<str>) -> Option<&Layer> {
		self.layers.get(id.as_ref())
	}
}

impl FromKdl<()> for Layout {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let default_layer = node.query_str_req("scope() > default_layer", 0)?.to_owned();

		let mut switches = HashMap::new();
		for mut node in node.query_all("scope() > switch")? {
			let name = node.next_str_req()?.to_owned();
			let switch = SwitchLocation::from_kdl(&mut node)?;
			switches.insert(name, switch);
		}

		let mut layers = HashMap::new();
		for mut node in node.query_all("scope() > layer")? {
			let name = node.next_str_req()?.to_owned();
			let layer = Layer::from_kdl(&mut node)?;
			layers.insert(name, layer);
		}

		Ok(Self {
			switches,
			default_layer,
			layers,
		})
	}
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layer {
	bindings: HashMap<String, KeyBinding>,
}

impl Layer {
	pub fn get_binding(&self, switch: impl AsRef<str>) -> Option<&KeyBinding> {
		self.bindings.get(switch.as_ref())
	}
}

impl FromKdl<()> for Layer {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let mut bindings = HashMap::new();
		for mut node in node.query_all("scope() > bind")? {
			let switch_id = node.next_str_req()?.to_owned();
			let binding = KeyBinding::from_kdl(&mut node)?;
			bindings.insert(switch_id, binding);
		}
		Ok(Self { bindings })
	}
}
