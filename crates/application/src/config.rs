use derivative::Derivative;
use kdlize::{ext::DocumentExt, AsKdl, FromKdl, OmitIfEmpty};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Mutex};

pub use global_hotkey::hotkey::{Code as HotKeyCode, HotKey as HotKey, Modifiers as HotKeyModifiers};

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

fn key_alias_to_code(alias: shared::KeyAlias) -> Option<HotKeyCode> {
	use shared::KeyAlias as Alias;
	match alias {
		Alias::Backquote => Some(HotKeyCode::Backquote),
		Alias::Backslash => Some(HotKeyCode::Backslash),
		Alias::BracketLeft => Some(HotKeyCode::BracketLeft),
		Alias::BracketRight => Some(HotKeyCode::BracketRight),
		Alias::Comma => Some(HotKeyCode::Comma),
		Alias::Digit0 => Some(HotKeyCode::Digit0),
		Alias::Digit1 => Some(HotKeyCode::Digit1),
		Alias::Digit2 => Some(HotKeyCode::Digit2),
		Alias::Digit3 => Some(HotKeyCode::Digit3),
		Alias::Digit4 => Some(HotKeyCode::Digit4),
		Alias::Digit5 => Some(HotKeyCode::Digit5),
		Alias::Digit6 => Some(HotKeyCode::Digit6),
		Alias::Digit7 => Some(HotKeyCode::Digit7),
		Alias::Digit8 => Some(HotKeyCode::Digit8),
		Alias::Digit9 => Some(HotKeyCode::Digit9),
		Alias::Equal => Some(HotKeyCode::Equal),
		Alias::KeyA => Some(HotKeyCode::KeyA),
		Alias::KeyB => Some(HotKeyCode::KeyB),
		Alias::KeyC => Some(HotKeyCode::KeyC),
		Alias::KeyD => Some(HotKeyCode::KeyD),
		Alias::KeyE => Some(HotKeyCode::KeyE),
		Alias::KeyF => Some(HotKeyCode::KeyF),
		Alias::KeyG => Some(HotKeyCode::KeyG),
		Alias::KeyH => Some(HotKeyCode::KeyH),
		Alias::KeyI => Some(HotKeyCode::KeyI),
		Alias::KeyJ => Some(HotKeyCode::KeyJ),
		Alias::KeyK => Some(HotKeyCode::KeyK),
		Alias::KeyL => Some(HotKeyCode::KeyL),
		Alias::KeyM => Some(HotKeyCode::KeyM),
		Alias::KeyN => Some(HotKeyCode::KeyN),
		Alias::KeyO => Some(HotKeyCode::KeyO),
		Alias::KeyP => Some(HotKeyCode::KeyP),
		Alias::KeyQ => Some(HotKeyCode::KeyQ),
		Alias::KeyR => Some(HotKeyCode::KeyR),
		Alias::KeyS => Some(HotKeyCode::KeyS),
		Alias::KeyT => Some(HotKeyCode::KeyT),
		Alias::KeyU => Some(HotKeyCode::KeyU),
		Alias::KeyV => Some(HotKeyCode::KeyV),
		Alias::KeyW => Some(HotKeyCode::KeyW),
		Alias::KeyX => Some(HotKeyCode::KeyX),
		Alias::KeyY => Some(HotKeyCode::KeyY),
		Alias::KeyZ => Some(HotKeyCode::KeyZ),
		Alias::Minus => Some(HotKeyCode::Minus),
		Alias::Period => Some(HotKeyCode::Period),
		Alias::Quote => Some(HotKeyCode::Quote),
		Alias::Semicolon => Some(HotKeyCode::Semicolon),
		Alias::Slash => Some(HotKeyCode::Slash),
		Alias::AltLeft => None, // not currently supported; Some(HotKeyCode::AltLeft),
		Alias::AltRight => None, // not currently supported; Some(HotKeyCode::AltRight),
		Alias::Backspace => Some(HotKeyCode::Backspace),
		Alias::CapsLock => Some(HotKeyCode::CapsLock),
		Alias::ControlLeft => None, // not currently supported; Some(HotKeyCode::ControlLeft),
		Alias::ControlRight => None, // not currently supported; Some(HotKeyCode::ControlRight),
		Alias::Enter => Some(HotKeyCode::Enter),
		Alias::MetaLeft => Some(HotKeyCode::MetaLeft),
		Alias::MetaRight => Some(HotKeyCode::MetaRight),
		Alias::ShiftLeft => None, // not currently supported; Some(HotKeyCode::ShiftLeft),
		Alias::ShiftRight => None, // not currently supported; Some(HotKeyCode::ShiftRight),
		Alias::Space => Some(HotKeyCode::Space),
		Alias::Tab => Some(HotKeyCode::Tab),
		Alias::Delete => Some(HotKeyCode::Delete),
		Alias::End => Some(HotKeyCode::End),
		Alias::Home => Some(HotKeyCode::Home),
		Alias::Insert => Some(HotKeyCode::Insert),
		Alias::PageDown => Some(HotKeyCode::PageDown),
		Alias::PageUp => Some(HotKeyCode::PageUp),
		Alias::ArrowDown => Some(HotKeyCode::ArrowDown),
		Alias::ArrowLeft => Some(HotKeyCode::ArrowLeft),
		Alias::ArrowRight => Some(HotKeyCode::ArrowRight),
		Alias::ArrowUp => Some(HotKeyCode::ArrowUp),
		Alias::Escape => Some(HotKeyCode::Escape),
		Alias::F1 => Some(HotKeyCode::F1),
		Alias::F2 => Some(HotKeyCode::F2),
		Alias::F3 => Some(HotKeyCode::F3),
		Alias::F4 => Some(HotKeyCode::F4),
		Alias::F5 => Some(HotKeyCode::F5),
		Alias::F6 => Some(HotKeyCode::F6),
		Alias::F7 => Some(HotKeyCode::F7),
		Alias::F8 => Some(HotKeyCode::F8),
		Alias::F9 => Some(HotKeyCode::F9),
		Alias::F10 => Some(HotKeyCode::F10),
		Alias::F11 => Some(HotKeyCode::F11),
		Alias::F12 => Some(HotKeyCode::F12),
		Alias::F13 => Some(HotKeyCode::F13),
		Alias::F14 => Some(HotKeyCode::F14),
		Alias::F15 => Some(HotKeyCode::F15),
		Alias::F16 => Some(HotKeyCode::F16),
		Alias::F17 => Some(HotKeyCode::F17),
		Alias::F18 => Some(HotKeyCode::F18),
		Alias::F19 => Some(HotKeyCode::F19),
		Alias::F20 => Some(HotKeyCode::F20),
		Alias::F21 => Some(HotKeyCode::F21),
		Alias::F22 => Some(HotKeyCode::F22),
		Alias::F23 => Some(HotKeyCode::F23),
		Alias::F24 => Some(HotKeyCode::F24),
		Alias::Fn => Some(HotKeyCode::Fn),
		Alias::FnLock => Some(HotKeyCode::FnLock),
		Alias::PrintScreen => Some(HotKeyCode::PrintScreen),
		Alias::ScrollLock => Some(HotKeyCode::ScrollLock),
		Alias::Pause => None, // not currently supported; Some(HotKeyCode::Pause),
		Alias::MediaPlayPause => Some(HotKeyCode::MediaPlayPause),
		Alias::MediaTrackNext => Some(HotKeyCode::MediaTrackNext),
		Alias::MediaTrackPrevious => Some(HotKeyCode::MediaTrackPrevious),
		Alias::AudioVolumeDown => Some(HotKeyCode::AudioVolumeDown),
		Alias::AudioVolumeMute => Some(HotKeyCode::AudioVolumeMute),
		Alias::AudioVolumeUp => Some(HotKeyCode::AudioVolumeUp),
		Alias::Tilde => None,
		Alias::Exclamation => None,
		Alias::At => None,
		Alias::Hash => None,
		Alias::Dollar => None,
		Alias::Percent => None,
		Alias::Caret => None,
		Alias::Ampersand => None,
		Alias::Star => None,
		Alias::ParenLeft => None,
		Alias::ParenRight => None,
		Alias::BraceLeft => None,
		Alias::BraceRight => None,
		Alias::Underscore => None,
		Alias::Plus => None,
		Alias::Pipe => None,
		Alias::Colon => None,
		Alias::QuoteDouble => None,
		Alias::LessThan => None,
		Alias::GreaterThan => None,
		Alias::Question => None,
	}
}

fn dealias_code(alias: shared::KeyAlias) -> Option<HotKeyCode> {
	use shared::KeyAlias as Alias;
	match alias {
		Alias::Tilde => Some(HotKeyCode::Digit0),
		Alias::Exclamation => Some(HotKeyCode::Digit1),
		Alias::At => Some(HotKeyCode::Digit2),
		Alias::Hash => Some(HotKeyCode::Digit3),
		Alias::Dollar => Some(HotKeyCode::Digit4),
		Alias::Percent => Some(HotKeyCode::Digit5),
		Alias::Caret => Some(HotKeyCode::Digit6),
		Alias::Ampersand => Some(HotKeyCode::Digit7),
		Alias::Star => Some(HotKeyCode::Digit8),
		Alias::ParenLeft => Some(HotKeyCode::Digit9),
		Alias::ParenRight => Some(HotKeyCode::Digit0),
		Alias::BraceLeft => Some(HotKeyCode::BracketLeft),
		Alias::BraceRight => Some(HotKeyCode::BracketRight),
		Alias::Underscore => Some(HotKeyCode::Minus),
		Alias::Plus => Some(HotKeyCode::Equal),
		Alias::Pipe => Some(HotKeyCode::Backslash),
		Alias::Colon => Some(HotKeyCode::Semicolon),
		Alias::QuoteDouble => Some(HotKeyCode::Quote),
		Alias::LessThan => Some(HotKeyCode::Comma),
		Alias::GreaterThan => Some(HotKeyCode::Period),
		Alias::Question => Some(HotKeyCode::Slash),
		_ => None,
	}
}

fn is_alpha(code: HotKeyCode) -> bool {
	static ALPHA: [HotKeyCode; 26] = [
		HotKeyCode::KeyA,
		HotKeyCode::KeyB,
		HotKeyCode::KeyC,
		HotKeyCode::KeyD,
		HotKeyCode::KeyE,
		HotKeyCode::KeyF,
		HotKeyCode::KeyG,
		HotKeyCode::KeyH,
		HotKeyCode::KeyI,
		HotKeyCode::KeyJ,
		HotKeyCode::KeyK,
		HotKeyCode::KeyL,
		HotKeyCode::KeyM,
		HotKeyCode::KeyN,
		HotKeyCode::KeyO,
		HotKeyCode::KeyP,
		HotKeyCode::KeyQ,
		HotKeyCode::KeyR,
		HotKeyCode::KeyS,
		HotKeyCode::KeyT,
		HotKeyCode::KeyU,
		HotKeyCode::KeyV,
		HotKeyCode::KeyW,
		HotKeyCode::KeyX,
		HotKeyCode::KeyY,
		HotKeyCode::KeyZ,
	];
	ALPHA.contains(&code)
}


pub fn alias_hotkeys(alias: shared::KeyAlias) -> Vec<HotKey> {
	let mut hotkeys = Vec::with_capacity(3);

	// Simple conversions, alias directly matches some code
	if let Some(code) = key_alias_to_code(alias) {
		hotkeys.push(HotKey::new(None, code));
		// Lower to Upper casings
		if is_alpha(code) {
			hotkeys.push(HotKey::new(Some(HotKeyModifiers::SHIFT), code));
		}
	}

	// Symbols which are represented by other codes
	if let Some(code) = dealias_code(alias) {
		hotkeys.push(HotKey::new(Some(HotKeyModifiers::SHIFT), code));
	}

	hotkeys
}
