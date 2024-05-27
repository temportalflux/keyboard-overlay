use crate::{Combo, Layer, Switch};
use kdlize::{ext::DocumentExt, AsKdl, FromKdl};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layout {
	switches: BTreeMap<String, Switch>,
	combos: Vec<Combo>,
	default_layer: String,
	layer_order: Vec<String>,
	layers: BTreeMap<String, Layer>,
}

impl Layout {
	pub fn default_layer(&self) -> &String {
		&self.default_layer
	}

	pub fn switches(&self) -> &BTreeMap<String, Switch> {
		&self.switches
	}

	pub fn combos(&self) -> &Vec<Combo> {
		&self.combos
	}

	pub fn get_layer(&self, id: impl AsRef<str>) -> Option<&Layer> {
		self.layers.get(id.as_ref())
	}

	pub fn layer_order(&self) -> &Vec<String> {
		&self.layer_order
	}

	pub fn layers(&self) -> &BTreeMap<String, Layer> {
		&self.layers
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

		let combos = node.query_all_t("scope() > combo")?;

		let mut layer_order = Vec::new();
		let mut layers = BTreeMap::new();
		for mut node in node.query_all("scope() > layer")? {
			let name = node.next_str_req()?.to_owned();
			let layer = Layer::from_kdl(&mut node)?;
			layer_order.push(name.clone());
			layers.insert(name, layer);
		}

		Ok(Self {
			switches,
			combos,
			default_layer,
			layer_order,
			layers,
		})
	}
}

impl AsKdl for Layout {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		node.child(("default_layer", &self.default_layer));
		for (name, switch) in &self.switches {
			node.child(("switch", &(name, switch)));
		}
		node.children(("combo", &self.combos));
		for name in &self.layer_order {
			let Some(layer) = self.layers.get(name) else { continue };
			node.child(("layer", &(name, layer)));
		}
		node
	}
}
