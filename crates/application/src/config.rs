use derivative::Derivative;
use kdlize::{ext::DocumentExt, FromKdl};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Default)]
pub struct ConfigMutex(Mutex<Config>);
impl ConfigMutex {
	pub fn get(&self) -> Config {
		self.0.lock().unwrap().clone()
	}

	pub fn set(&self, value: Config) {
		*self.0.lock().unwrap() = value;
	}
}

pub fn load_config(app_config: &tauri::Config) -> anyhow::Result<Option<Config>> {
	let Some(config_path) = tauri::api::path::app_config_dir(&app_config) else {
		return Ok(None);
	};
	let config_path = config_path.join("config.kdl");
	// TODO: generate a default config.kdl if one does not exist
	if !config_path.exists() {
		return Ok(None);
	}
	let config_str = tauri::api::file::read_string(config_path)?;
	let config_doc = config_str.parse::<kdl::KdlDocument>()?;
	let mut doc_node = kdl::KdlNode::new("document");
	doc_node.set_children(config_doc);
	let mut node = kdlize::NodeReader::new_root(&doc_node, ());
	let config = Config::from_kdl(&mut node)?;
	Ok(Some(config))
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Config {
	pub size: (u32, u32),
	pub location: WindowPosition,
	pub layout: shared::Layout,
}

impl FromKdl<()> for Config {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let size = {
			let mut node = node.query_req("scope() > size")?;
			let w = node.next_i64_req()? as u32;
			let h = node.next_i64_req()? as u32;
			(w, h)
		};
		let location = node.query_req_t("scope() > location")?;		
		let layout = node.query_req_t("scope() > layout")?;
		Ok(Self { size, location, layout })
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WindowPosition {
	pub monitor: usize,
	pub anchor: WindowAnchor,
	pub offset: (i32, i32),
}

impl FromKdl<()> for WindowPosition {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let monitor = node.query_i64_opt("scope() > monitor", 0)?;
		let monitor = monitor.map(|idx| (idx - 1) as usize).unwrap_or_default();
		let anchor = node.query_str_req_t("scope() > anchor", 0)?;
		let offset = {
			let mut node = node.query_req("scope() > offset")?;
			let w = node.next_i64_req()? as i32;
			let h = node.next_i64_req()? as i32;
			(w, h)
		};
		Ok(Self {
			monitor,
			anchor,
			offset,
		})
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Derivative, Serialize, Deserialize)]
#[derivative(Default)]
pub enum WindowAnchor {
	TopLeft,
	TopCenter,
	TopRight,
	BottomLeft,
	BottomCenter,
	BottomRight,
	CenterLeft,
	#[derivative(Default)]
	Center,
	CenterRight,
}
impl Into<tauri_plugin_positioner::Position> for WindowAnchor {
	fn into(self) -> tauri_plugin_positioner::Position {
		match self {
			Self::TopLeft => tauri_plugin_positioner::Position::TopLeft,
			Self::TopCenter => tauri_plugin_positioner::Position::TopCenter,
			Self::TopRight => tauri_plugin_positioner::Position::TopRight,
			Self::BottomLeft => tauri_plugin_positioner::Position::BottomLeft,
			Self::BottomCenter => tauri_plugin_positioner::Position::BottomCenter,
			Self::BottomRight => tauri_plugin_positioner::Position::BottomRight,
			Self::CenterLeft => tauri_plugin_positioner::Position::LeftCenter,
			Self::Center => tauri_plugin_positioner::Position::Center,
			Self::CenterRight => tauri_plugin_positioner::Position::RightCenter,
		}
	}
}
impl std::str::FromStr for WindowAnchor {
	type Err = InvalidWindowAnchor;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"TopLeft" => Ok(Self::TopLeft),
			"TopCenter" => Ok(Self::TopCenter),
			"TopRight" => Ok(Self::TopRight),
			"BottomLeft" => Ok(Self::BottomLeft),
			"BottomCenter" => Ok(Self::BottomCenter),
			"BottomRight" => Ok(Self::BottomRight),
			"CenterLeft" => Ok(Self::CenterLeft),
			"Center" => Ok(Self::Center),
			"CenterRight" => Ok(Self::CenterRight),
			_ => Err(InvalidWindowAnchor(s.to_owned())),
		}
	}
}

#[derive(thiserror::Error, Debug)]
#[error("Invalid window anchor {0:?}")]
pub struct InvalidWindowAnchor(String);
