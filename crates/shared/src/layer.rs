use crate::BoundSwitch;
use kdlize::{AsKdl, FromKdl};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
