use crate::{KeySet, SwitchSlot};
use kdlize::{ext::ValueExt, AsKdl, FromKdl};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
	pub input: KeySet,
	pub display: Option<BindingDisplay>,
	pub layer: Option<String>,
}

impl FromKdl<()> for Binding {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let input = node.next_str_req_t::<KeySet>()?;
		let display = match node.next_opt() {
			None => None,
			Some(entry) => Some(BindingDisplay::try_from(entry)?),
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BindingDisplay {
	Text(String),
	IconBootstrap(String),
	IconCustom(String),
}

#[derive(thiserror::Error, Debug)]
#[error("Invalid binding display type {0}, expecting IconBootstrap or IconCustom")]
pub struct InvalidBindingDisplay(String);

impl TryFrom<&kdl::KdlEntry> for BindingDisplay {
	type Error = anyhow::Error;

	fn try_from(entry: &kdl::KdlEntry) -> Result<Self, Self::Error> {
		let value = entry.as_str_req()?.to_owned();
		match entry.ty() {
			None => Ok(BindingDisplay::Text(value)),
			Some(kind_str) => match kind_str.value() {
				"IconBootstrap" => Ok(BindingDisplay::IconBootstrap(value)),
				"IconCustom" => Ok(BindingDisplay::IconCustom(value)),
				kind_id => Err(InvalidBindingDisplay(kind_id.to_owned()))?,
			},
		}
	}
}

impl AsKdl for BindingDisplay {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		match self {
			BindingDisplay::Text(value) => node.entry(value.as_str()),
			BindingDisplay::IconBootstrap(value) => node.entry_typed("IconBootstrap", value.as_str()),
			BindingDisplay::IconCustom(value) => node.entry_typed("IconCustom", value.as_str()),
		}
		node
	}
}
