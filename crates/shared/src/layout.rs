use crate::{KeyCombo, Switch};
use kdlize::{
	ext::{DocumentExt, ValueExt},
	AsKdl, FromKdl,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layout {
	switches: BTreeMap<String, Switch>,
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
		for name in &self.layer_order {
			let Some(layer) = self.layers.get(name) else { continue };
			node.child(("layer", &(name, layer)));
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
				.with(binding.as_kdl());
			node.child(node_binding.build("bind"));
		}
		node
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SwitchSlot {
	Tap,
	Hold,
}

#[derive(thiserror::Error, Debug)]
#[error("Invalid switch slot {0}, expectd Tap or Hold")]
pub struct InvalidSlot(String);

impl std::str::FromStr for SwitchSlot {
	type Err = InvalidSlot;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"Tap" => Ok(Self::Tap),
			"Hold" => Ok(Self::Hold),
			_ => Err(InvalidSlot(s.to_owned())),
		}
	}
}

impl std::fmt::Display for SwitchSlot {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::Tap => "Tap",
				Self::Hold => "Hold",
			}
		)
	}
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BoundSwitch {
	pub slots: BTreeMap<SwitchSlot, Binding>,
}

impl FromKdl<()> for BoundSwitch {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let mut slots = BTreeMap::new();
		for mut node in node.query_all("scope() > slot")? {
			let slot = node.next_str_req_t::<SwitchSlot>()?;
			let binding = Binding::from_kdl(&mut node)?;
			slots.insert(slot, binding);
		}
		Ok(Self { slots })
	}
}

impl AsKdl for BoundSwitch {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		for (slot, binding) in &self.slots {
			node.child(
				kdlize::NodeBuilder::default()
					.with_entry(slot.to_string())
					.with(binding.as_kdl())
					.build("slot"),
			);
		}
		node
	}
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Binding {
	pub input: KeyCombo,
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
		let input = node.next_str_req_t::<KeyCombo>()?;
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
					},
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
		node.entry(self.input.to_string());
		if let Some(display) = &self.display {
			match display {
				BindingDisplay::Text(value) => node.entry(value.as_str()),
				BindingDisplay::IconBootstrap(value) => node.entry_typed("IconBootstrap", value.as_str()),
				BindingDisplay::IconCustom(value) => node.entry_typed("IconCustom", value.as_str()),
			}
		}
		node.entry(("layer", self.layer.clone()));
		node
	}
}
