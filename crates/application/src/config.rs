use derivative::Derivative;
use kdlize::{ext::DocumentExt, AsKdl, FromKdl, OmitIfEmpty};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Mutex};

// TODO: multiple layouts (consider naming layouts? figure out how to associate them with different keyboards)
// TODO: load from url

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
	if !config_path.exists() {
		return Ok(None);
	}
	let config_str = tauri::api::file::read_string(config_path)?;
	let config = parse_config_kdl(&config_str)?;
	Ok(Some(config))
}

pub fn parse_config_kdl(config_str: &str) -> Result<Config, <Config as FromKdl<()>>::Error> {
	let config_doc = config_str.parse::<kdl::KdlDocument>()?;
	let mut doc_node = kdl::KdlNode::new("document");
	doc_node.set_children(config_doc);
	let mut node = kdlize::NodeReader::new_root(&doc_node, ());
	let config = Config::from_kdl(&mut node)?;
	Ok(config)
}

pub fn serialize_config_kdl(config: &Config) -> String {
	let contents = config.as_kdl().into_document().to_string();
	let contents = contents.replace("    ", "\t");
	contents
}

pub fn save_config(app_config: &tauri::Config, config: &Config) -> anyhow::Result<()> {
	let Some(config_path) = tauri::api::path::app_config_dir(&app_config) else {
		return Ok(());
	};
	std::fs::create_dir_all(&config_path)?;
	let config_path = config_path.join("config.kdl");
	std::fs::write(config_path, serialize_config_kdl(config))?;
	Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
	default_profile: String,
	active_profile: String,
	profiles: BTreeMap<String, DisplayProfile>,
	layout: shared::Layout,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			default_profile: "default".into(),
			active_profile: "default".into(),
			profiles: [(
				"default".into(),
				DisplayProfile {
					size: (800, 600),
					scale: 1.0,
					location: WindowPosition {
						anchor: WindowAnchor::Center,
						monitor: 0,
						offset: (0, 0),
					},
				},
			)]
			.into(),
			layout: shared::Layout::default(),
		}
	}
}

impl Config {
	pub fn default_profile_id(&self) -> &String {
		&self.default_profile
	}

	pub fn active_profile(&self) -> Option<&DisplayProfile> {
		self.profile(&self.active_profile)
	}

	pub fn set_active_profile(&mut self, name: impl AsRef<str>) -> Result<(), anyhow::Error> {
		if !self.profiles.contains_key(name.as_ref()) {
			return Err(anyhow::Error::msg("Invalid profile name"));
		}
		self.active_profile = name.as_ref().to_owned();
		Ok(())
	}

	pub fn has_profiles(&self) -> bool {
		!self.profiles.is_empty()
	}

	pub fn iter_profiles(&self) -> impl Iterator<Item = (&String, &DisplayProfile)> + '_ {
		self.profiles.iter()
	}

	pub fn profile(&self, key: impl AsRef<str>) -> Option<&DisplayProfile> {
		self.profiles.get(key.as_ref())
	}

	pub fn layout(&self) -> &shared::Layout {
		&self.layout
	}

	pub fn clear_state(&mut self) {
		self.active_profile.clear();
	}
}

impl FromKdl<()> for Config {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let default_profile = node.query_str_req("scope() > default_profile", 0)?.to_owned();
		let active_profile = node.query_str_opt("scope() > active_profile", 0)?;
		let active_profile = active_profile
			.map(str::to_owned)
			.unwrap_or_else(|| default_profile.clone());

		let mut profiles = BTreeMap::new();
		for mut node in node.query_all("scope() > profile")? {
			let name = node.next_str_req()?.to_owned();
			let layer = DisplayProfile::from_kdl(&mut node)?;
			profiles.insert(name, layer);
		}

		let layout = node.query_req_t("scope() > layout")?;

		Ok(Self {
			default_profile,
			active_profile,
			profiles,
			layout,
		})
	}
}

impl AsKdl for Config {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		node.push_child_t(("default_profile", &self.default_profile));
		node.push_child_t(("active_profile", &self.active_profile, OmitIfEmpty));
		for (name, profile) in &self.profiles {
			node.push_child_t(("profile", &(name, profile)));
		}
		node.push_child_t(("layout", &self.layout));
		node
	}
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DisplayProfile {
	pub size: (u32, u32),
	pub location: WindowPosition,
	pub scale: f64,
}

impl FromKdl<()> for DisplayProfile {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let size = {
			let mut node = node.query_req("scope() > size")?;
			let w = node.next_i64_req()? as u32;
			let h = node.next_i64_req()? as u32;
			(w, h)
		};
		let location = node.query_req_t("scope() > location")?;
		let scale = node.query_f64_opt("scope() > scale", 0)?.unwrap_or(1.0);
		Ok(Self { size, scale, location })
	}
}

impl AsKdl for DisplayProfile {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		node.push_child({
			let mut node = kdlize::NodeBuilder::default();
			node.push_entry(self.size.0 as i64);
			node.push_entry(self.size.1 as i64);
			node.build("size")
		});
		if self.scale != 1.0 {
			node.push_child_t(("scale", &self.scale));
		}
		node.push_child_t(("location", &self.location));
		node
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

impl AsKdl for WindowPosition {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		if self.monitor != 0 {
			node.push_child_t(("monitor", &(self.monitor + 1)));
		}
		node.push_child_t(("anchor", &self.anchor.to_string()));
		node.push_child({
			let mut node = kdlize::NodeBuilder::default();
			node.push_entry(self.offset.0 as i64);
			node.push_entry(self.offset.1 as i64);
			node.build("offset")
		});
		node
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
impl std::fmt::Display for WindowAnchor {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::TopLeft => "TopLeft",
				Self::TopCenter => "TopCenter",
				Self::TopRight => "TopRight",
				Self::BottomLeft => "BottomLeft",
				Self::BottomCenter => "BottomCenter",
				Self::BottomRight => "BottomRight",
				Self::CenterLeft => "CenterLeft",
				Self::Center => "Center",
				Self::CenterRight => "CenterRight",
			}
		)
	}
}

#[derive(thiserror::Error, Debug)]
#[error("Invalid window anchor {0:?}")]
pub struct InvalidWindowAnchor(String);

pub fn switch_key_to_rdev(binding: shared::OSKey) -> rdev::Key {
	match binding {
		shared::OSKey::F1 => rdev::Key::F1,
		shared::OSKey::F2 => rdev::Key::F2,
		shared::OSKey::F3 => rdev::Key::F3,
		shared::OSKey::F4 => rdev::Key::F4,
		shared::OSKey::F5 => rdev::Key::F5,
		shared::OSKey::F6 => rdev::Key::F6,
		shared::OSKey::F7 => rdev::Key::F7,
		shared::OSKey::F8 => rdev::Key::F8,
		shared::OSKey::F9 => rdev::Key::F9,
		shared::OSKey::F10 => rdev::Key::F10,
		shared::OSKey::F11 => rdev::Key::F11,
		shared::OSKey::F12 => rdev::Key::F12,

		shared::OSKey::Function => rdev::Key::Function,
		shared::OSKey::AltLeft => rdev::Key::Alt,
		shared::OSKey::AltRight => rdev::Key::AltGr,
		shared::OSKey::ControlLeft => rdev::Key::ControlLeft,
		shared::OSKey::ControlRight => rdev::Key::ControlRight,
		shared::OSKey::ShiftLeft => rdev::Key::ShiftLeft,
		shared::OSKey::ShiftRight => rdev::Key::ShiftRight,
		shared::OSKey::MetaLeft => rdev::Key::MetaLeft,
		shared::OSKey::MetaRight => rdev::Key::MetaRight,

		shared::OSKey::CapsLock => rdev::Key::CapsLock,
		shared::OSKey::NumLock => rdev::Key::NumLock,
		shared::OSKey::ScrollLock => rdev::Key::ScrollLock,

		shared::OSKey::Tab => rdev::Key::Tab,
		shared::OSKey::Space => rdev::Key::Space,
		shared::OSKey::Backspace => rdev::Key::Backspace,
		shared::OSKey::Delete => rdev::Key::Delete,
		shared::OSKey::Return => rdev::Key::Return,
		shared::OSKey::Escape => rdev::Key::Escape,

		shared::OSKey::LeftArrow => rdev::Key::LeftArrow,
		shared::OSKey::DownArrow => rdev::Key::DownArrow,
		shared::OSKey::RightArrow => rdev::Key::RightArrow,
		shared::OSKey::UpArrow => rdev::Key::UpArrow,

		shared::OSKey::Insert => rdev::Key::Insert,
		shared::OSKey::Home => rdev::Key::Home,
		shared::OSKey::End => rdev::Key::End,
		shared::OSKey::PageUp => rdev::Key::PageUp,
		shared::OSKey::PageDown => rdev::Key::PageDown,
		shared::OSKey::PrintScreen => rdev::Key::PrintScreen,
		shared::OSKey::Pause => rdev::Key::Pause,

		shared::OSKey::Num1 => rdev::Key::Num1,
		shared::OSKey::Num2 => rdev::Key::Num2,
		shared::OSKey::Num3 => rdev::Key::Num3,
		shared::OSKey::Num4 => rdev::Key::Num4,
		shared::OSKey::Num5 => rdev::Key::Num5,
		shared::OSKey::Num6 => rdev::Key::Num6,
		shared::OSKey::Num7 => rdev::Key::Num7,
		shared::OSKey::Num8 => rdev::Key::Num8,
		shared::OSKey::Num9 => rdev::Key::Num9,
		shared::OSKey::Num0 => rdev::Key::Num0,

		shared::OSKey::Minus => rdev::Key::Minus,
		shared::OSKey::Equal => rdev::Key::Equal,
		shared::OSKey::SemiColon => rdev::Key::SemiColon,
		shared::OSKey::Comma => rdev::Key::Comma,
		shared::OSKey::Dot => rdev::Key::Dot,
		shared::OSKey::Quote => rdev::Key::Quote,
		shared::OSKey::Grave => rdev::Key::BackQuote,
		shared::OSKey::Slash => rdev::Key::Slash,
		shared::OSKey::BackSlash => rdev::Key::BackSlash,
		shared::OSKey::IntlBackslash => rdev::Key::IntlBackslash,
		shared::OSKey::BracketLeft => rdev::Key::LeftBracket,
		shared::OSKey::BracketRight => rdev::Key::RightBracket,

		shared::OSKey::KeyQ => rdev::Key::KeyQ,
		shared::OSKey::KeyW => rdev::Key::KeyW,
		shared::OSKey::KeyE => rdev::Key::KeyE,
		shared::OSKey::KeyR => rdev::Key::KeyR,
		shared::OSKey::KeyT => rdev::Key::KeyT,
		shared::OSKey::KeyY => rdev::Key::KeyY,
		shared::OSKey::KeyU => rdev::Key::KeyU,
		shared::OSKey::KeyI => rdev::Key::KeyI,
		shared::OSKey::KeyO => rdev::Key::KeyO,
		shared::OSKey::KeyP => rdev::Key::KeyP,
		shared::OSKey::KeyA => rdev::Key::KeyA,
		shared::OSKey::KeyS => rdev::Key::KeyS,
		shared::OSKey::KeyD => rdev::Key::KeyD,
		shared::OSKey::KeyF => rdev::Key::KeyF,
		shared::OSKey::KeyG => rdev::Key::KeyG,
		shared::OSKey::KeyH => rdev::Key::KeyH,
		shared::OSKey::KeyJ => rdev::Key::KeyJ,
		shared::OSKey::KeyK => rdev::Key::KeyK,
		shared::OSKey::KeyL => rdev::Key::KeyL,
		shared::OSKey::KeyZ => rdev::Key::KeyZ,
		shared::OSKey::KeyX => rdev::Key::KeyX,
		shared::OSKey::KeyC => rdev::Key::KeyC,
		shared::OSKey::KeyV => rdev::Key::KeyV,
		shared::OSKey::KeyB => rdev::Key::KeyB,
		shared::OSKey::KeyN => rdev::Key::KeyN,
		shared::OSKey::KeyM => rdev::Key::KeyM,

		shared::OSKey::Keypad0 => rdev::Key::Kp0,
		shared::OSKey::Keypad1 => rdev::Key::Kp1,
		shared::OSKey::Keypad2 => rdev::Key::Kp2,
		shared::OSKey::Keypad3 => rdev::Key::Kp3,
		shared::OSKey::Keypad4 => rdev::Key::Kp4,
		shared::OSKey::Keypad5 => rdev::Key::Kp5,
		shared::OSKey::Keypad6 => rdev::Key::Kp6,
		shared::OSKey::Keypad7 => rdev::Key::Kp7,
		shared::OSKey::Keypad8 => rdev::Key::Kp8,
		shared::OSKey::Keypad9 => rdev::Key::Kp9,
		shared::OSKey::KeypadMinus => rdev::Key::KpMinus,
		shared::OSKey::KeypadPlus => rdev::Key::KpPlus,
		shared::OSKey::KeypadMultiply => rdev::Key::KpMultiply,
		shared::OSKey::KeypadDivide => rdev::Key::KpDivide,
		shared::OSKey::KeypadReturn => rdev::Key::KpReturn,
		shared::OSKey::KeypadDelete => rdev::Key::KpDelete,

		shared::OSKey::Unknown(value) => rdev::Key::Unknown(value),
	}
}
