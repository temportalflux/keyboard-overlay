use crate::Switch;
use kdlize::{ext::DocumentExt, AsKdl, FromKdl};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layout {
	switches: BTreeMap<String, Switch>,
	default_layer: String,
	layers: BTreeMap<String, Layer>,
}

impl Layout {
	pub fn default_layer(&self) -> &String {
		&self.default_layer
	}

	pub fn switches(&self) -> &BTreeMap<String, Switch> {
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

		let mut switches = BTreeMap::new();
		for mut node in node.query_all("scope() > switch")? {
			let name = node.next_str_req()?.to_owned();
			let switch = Switch::from_kdl(&mut node)?;
			switches.insert(name, switch);
		}

		let mut layers = BTreeMap::new();
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

impl AsKdl for Layout {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		node.push_child_t(("default_layer", &self.default_layer));
		for (name, switch) in &self.switches {
			node.push_child_t(("switch", &(name, switch)));
		}
		for (name, layer) in &self.layers {
			node.push_child_t(("layer", &(name, layer)));
		}
		node
	}
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layer {
	bindings: BTreeMap<String, String>,
}

impl Layer {
	pub fn get_binding(&self, switch: impl AsRef<str>) -> Option<&String> {
		self.bindings.get(switch.as_ref())
	}
}

impl FromKdl<()> for Layer {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let mut bindings = BTreeMap::new();
		for mut node in node.query_all("scope() > bind")? {
			let switch_id = node.next_str_req()?.to_owned();
			let binding = node.next_str_req()?.to_owned();
			bindings.insert(switch_id, binding);
		}
		Ok(Self { bindings })
	}
}

impl AsKdl for Layer {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		for (switch_id, binding) in &self.bindings {
			node.push_child_t(("bind", &(switch_id, binding)));
		}
		node
	}
}
