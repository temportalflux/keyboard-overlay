use crate::{Switch, KeyAlias};
use kdlize::{ext::{DocumentExt, ValueExt}, AsKdl, FromKdl};
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
	bindings: BTreeMap<String, BoundSwitch>,
}

impl Layer {
	pub fn bindings(&self) -> &BTreeMap<String, BoundSwitch> {
		&self.bindings
	}

	pub fn get_binding(&self, switch: impl AsRef<str>) -> Option<&BoundSwitch> {
		self.bindings.get(switch.as_ref())
	}
}

impl FromKdl<()> for Layer {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let mut bindings = BTreeMap::new();
		for mut node in node.query_all("scope() > bind")? {
			let switch_id = node.next_str_req()?.to_owned();
			let binding = BoundSwitch::from_kdl(&mut node)?;
			bindings.insert(switch_id, binding);
		}
		Ok(Self { bindings })
	}
}

impl AsKdl for Layer {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		for (switch_id, binding) in &self.bindings {
			let node_binding = kdlize::NodeBuilder::default()
				.with_entry(switch_id.as_str())
				.with_extension(binding.as_kdl());
			node.push_child(node_binding.build("bind"));
		}
		node
	}
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BoundSwitch {
	pub tap: Option<Binding>,
	pub hold: Option<Binding>,
}

impl FromKdl<()> for BoundSwitch {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let tap = node.query_opt_t("scope() > tap")?;
		let hold = node.query_opt_t("scope() > hold")?;
		Ok(Self { tap, hold })
	}
}

impl AsKdl for BoundSwitch {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		node.push_child_t(("tap", &self.tap));
		node.push_child_t(("hold", &self.hold));
		node
	}
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Binding {
	pub input: KeyAlias,
	pub display: Option<BindingDisplay>,
	pub layer: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BindingDisplay {
	Text(String),
	IconBootstrap(String),
	IconCustom(String),
}

#[derive(thiserror::Error, Debug)]
#[error("Invalid binding display type {0}, expecting IconBootstrap or IconCustom")]
pub struct InvalidBindingDisplay(String);

impl FromKdl<()> for Binding {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let input = node.next_str_req_t::<KeyAlias>()?;
		let display = match node.next_opt() {
			None => None,
			Some(entry) => {
				let value = entry.as_str_req()?.to_owned();
				match entry.ty() {
					None => Some(BindingDisplay::Text(value)),
					Some(kind_str) => match kind_str.value() {
						"IconBootstrap" => Some(BindingDisplay::IconBootstrap(value)),
						"IconCustom" => Some(BindingDisplay::IconCustom(value)),
						kind_id => Err(InvalidBindingDisplay(kind_id.to_owned()))?,
					}
				}
			}
		};
		let layer = node.get_str_opt("layer")?.map(str::to_owned);
		Ok(Self { input, display, layer })
	}
}

impl AsKdl for Binding {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		node.push_entry(self.input.to_string());
		if let Some(display) = &self.display {
			match display {
				BindingDisplay::Text(value) => node.push_entry(value.as_str()),
				BindingDisplay::IconBootstrap(value) => node.push_entry_typed("IconBootstrap", value.as_str()),
				BindingDisplay::IconCustom(value) => node.push_entry_typed("IconCustom", value.as_str()),
			}
		}
		node.push_entry(("layer", self.layer.clone()));
		node
	}
}
