use derivative::Derivative;
use kdlize::{ext::DocumentExt, AsKdl, FromKdl, OmitIfEmpty};
use serde::{Deserialize, Serialize};
use std::{
	collections::{BTreeMap, HashSet},
	sync::Mutex,
};

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
		node.child(("default_profile", &self.default_profile));
		node.child(("active_profile", &self.active_profile, OmitIfEmpty));
		for (name, profile) in &self.profiles {
			node.child(("profile", &(name, profile)));
		}
		node.child(("layout", &self.layout));
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
		node.child({
			let mut node = kdlize::NodeBuilder::default();
			node.entry(self.size.0 as i64);
			node.entry(self.size.1 as i64);
			node.build("size")
		});
		if self.scale != 1.0 {
			node.child(("scale", &self.scale));
		}
		node.child(("location", &self.location));
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
			node.child(("monitor", &(self.monitor + 1)));
		}
		node.child(("anchor", &self.anchor.to_string()));
		node.child({
			let mut node = kdlize::NodeBuilder::default();
			node.entry(self.offset.0 as i64);
			node.entry(self.offset.1 as i64);
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

fn key_alias_to_code(alias: shared::KeyAlias) -> Option<rdev::Key> {
	use shared::KeyAlias as Alias;
	match alias {
		Alias::Backquote => Some(rdev::Key::BackQuote),
		Alias::Backslash => Some(rdev::Key::BackSlash),
		Alias::BracketLeft => Some(rdev::Key::LeftBracket),
		Alias::BracketRight => Some(rdev::Key::RightBracket),
		Alias::Comma => Some(rdev::Key::Comma),
		Alias::Digit0 => Some(rdev::Key::Num0),
		Alias::Digit1 => Some(rdev::Key::Num1),
		Alias::Digit2 => Some(rdev::Key::Num2),
		Alias::Digit3 => Some(rdev::Key::Num3),
		Alias::Digit4 => Some(rdev::Key::Num4),
		Alias::Digit5 => Some(rdev::Key::Num5),
		Alias::Digit6 => Some(rdev::Key::Num6),
		Alias::Digit7 => Some(rdev::Key::Num7),
		Alias::Digit8 => Some(rdev::Key::Num8),
		Alias::Digit9 => Some(rdev::Key::Num9),
		Alias::Equal => Some(rdev::Key::Equal),
		Alias::KeyA => Some(rdev::Key::KeyA),
		Alias::KeyB => Some(rdev::Key::KeyB),
		Alias::KeyC => Some(rdev::Key::KeyC),
		Alias::KeyD => Some(rdev::Key::KeyD),
		Alias::KeyE => Some(rdev::Key::KeyE),
		Alias::KeyF => Some(rdev::Key::KeyF),
		Alias::KeyG => Some(rdev::Key::KeyG),
		Alias::KeyH => Some(rdev::Key::KeyH),
		Alias::KeyI => Some(rdev::Key::KeyI),
		Alias::KeyJ => Some(rdev::Key::KeyJ),
		Alias::KeyK => Some(rdev::Key::KeyK),
		Alias::KeyL => Some(rdev::Key::KeyL),
		Alias::KeyM => Some(rdev::Key::KeyM),
		Alias::KeyN => Some(rdev::Key::KeyN),
		Alias::KeyO => Some(rdev::Key::KeyO),
		Alias::KeyP => Some(rdev::Key::KeyP),
		Alias::KeyQ => Some(rdev::Key::KeyQ),
		Alias::KeyR => Some(rdev::Key::KeyR),
		Alias::KeyS => Some(rdev::Key::KeyS),
		Alias::KeyT => Some(rdev::Key::KeyT),
		Alias::KeyU => Some(rdev::Key::KeyU),
		Alias::KeyV => Some(rdev::Key::KeyV),
		Alias::KeyW => Some(rdev::Key::KeyW),
		Alias::KeyX => Some(rdev::Key::KeyX),
		Alias::KeyY => Some(rdev::Key::KeyY),
		Alias::KeyZ => Some(rdev::Key::KeyZ),
		Alias::Minus => Some(rdev::Key::Minus),
		Alias::Period => Some(rdev::Key::Dot),
		Alias::Quote => Some(rdev::Key::Quote),
		Alias::Semicolon => Some(rdev::Key::SemiColon),
		Alias::Slash => Some(rdev::Key::Slash),
		Alias::AltLeft => Some(rdev::Key::Alt),
		Alias::AltRight => Some(rdev::Key::AltGr),
		Alias::Backspace => Some(rdev::Key::Backspace),
		Alias::CapsLock => Some(rdev::Key::CapsLock),
		Alias::ControlLeft => Some(rdev::Key::ControlLeft),
		Alias::ControlRight => Some(rdev::Key::ControlRight),
		Alias::Enter => Some(rdev::Key::Return),
		Alias::MetaLeft => Some(rdev::Key::MetaLeft),
		Alias::MetaRight => Some(rdev::Key::MetaRight),
		Alias::ShiftLeft => Some(rdev::Key::ShiftLeft),
		Alias::ShiftRight => Some(rdev::Key::ShiftRight),
		Alias::Space => Some(rdev::Key::Space),
		Alias::Tab => Some(rdev::Key::Tab),
		Alias::Delete => Some(rdev::Key::Delete),
		Alias::End => Some(rdev::Key::End),
		Alias::Home => Some(rdev::Key::Home),
		Alias::Insert => Some(rdev::Key::Insert),
		Alias::PageDown => Some(rdev::Key::PageDown),
		Alias::PageUp => Some(rdev::Key::PageUp),
		Alias::ArrowDown => Some(rdev::Key::DownArrow),
		Alias::ArrowLeft => Some(rdev::Key::LeftArrow),
		Alias::ArrowRight => Some(rdev::Key::RightArrow),
		Alias::ArrowUp => Some(rdev::Key::UpArrow),
		Alias::Escape => Some(rdev::Key::Escape),
		Alias::F1 => Some(rdev::Key::F1),
		Alias::F2 => Some(rdev::Key::F2),
		Alias::F3 => Some(rdev::Key::F3),
		Alias::F4 => Some(rdev::Key::F4),
		Alias::F5 => Some(rdev::Key::F5),
		Alias::F6 => Some(rdev::Key::F6),
		Alias::F7 => Some(rdev::Key::F7),
		Alias::F8 => Some(rdev::Key::F8),
		Alias::F9 => Some(rdev::Key::F9),
		Alias::F10 => Some(rdev::Key::F10),
		Alias::F11 => Some(rdev::Key::F11),
		Alias::F12 => Some(rdev::Key::F12),
		Alias::F13 => Some(rdev::Key::Unknown(124)),
		Alias::F14 => Some(rdev::Key::Unknown(125)),
		Alias::F15 => Some(rdev::Key::Unknown(126)),
		Alias::F16 => Some(rdev::Key::Unknown(127)),
		Alias::F17 => Some(rdev::Key::Unknown(128)),
		Alias::F18 => Some(rdev::Key::Unknown(129)),
		Alias::F19 => Some(rdev::Key::Unknown(130)),
		Alias::F20 => Some(rdev::Key::Unknown(131)),
		Alias::F21 => Some(rdev::Key::Unknown(132)),
		Alias::F22 => Some(rdev::Key::Unknown(133)),
		Alias::F23 => Some(rdev::Key::Unknown(134)),
		Alias::F24 => Some(rdev::Key::Unknown(135)),
		Alias::Fn => Some(rdev::Key::Function),
		Alias::PrintScreen => Some(rdev::Key::PrintScreen),
		Alias::ScrollLock => Some(rdev::Key::ScrollLock),
		Alias::Pause => Some(rdev::Key::Pause),
		Alias::MediaPlayPause => Some(rdev::Key::Unknown(179)),
		Alias::MediaTrackNext => Some(rdev::Key::Unknown(176)),
		Alias::MediaTrackPrevious => Some(rdev::Key::Unknown(177)),
		Alias::AudioVolumeDown => Some(rdev::Key::Unknown(174)),
		Alias::AudioVolumeMute => Some(rdev::Key::Unknown(173)),
		Alias::AudioVolumeUp => Some(rdev::Key::Unknown(175)),
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

fn dealias_code(alias: shared::KeyAlias) -> Option<rdev::Key> {
	use shared::KeyAlias as Alias;
	match alias {
		Alias::Tilde => Some(rdev::Key::Num0),
		Alias::Exclamation => Some(rdev::Key::Num1),
		Alias::At => Some(rdev::Key::Num2),
		Alias::Hash => Some(rdev::Key::Num3),
		Alias::Dollar => Some(rdev::Key::Num4),
		Alias::Percent => Some(rdev::Key::Num5),
		Alias::Caret => Some(rdev::Key::Num6),
		Alias::Ampersand => Some(rdev::Key::Num7),
		Alias::Star => Some(rdev::Key::Num8),
		Alias::ParenLeft => Some(rdev::Key::Num9),
		Alias::ParenRight => Some(rdev::Key::Num0),
		Alias::BraceLeft => Some(rdev::Key::LeftBracket),
		Alias::BraceRight => Some(rdev::Key::RightBracket),
		Alias::Underscore => Some(rdev::Key::Minus),
		Alias::Plus => Some(rdev::Key::Equal),
		Alias::Pipe => Some(rdev::Key::BackSlash),
		Alias::Colon => Some(rdev::Key::SemiColon),
		Alias::QuoteDouble => Some(rdev::Key::Quote),
		Alias::LessThan => Some(rdev::Key::Comma),
		Alias::GreaterThan => Some(rdev::Key::Dot),
		Alias::Question => Some(rdev::Key::Slash),
		_ => None,
	}
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct HotKey {
	pub code: rdev::Key,
	pub shift: bool,
	pub ctrl: bool,
	pub alt: bool,
	pub meta: bool,
}
impl Default for HotKey {
	fn default() -> Self {
		Self {
			code: rdev::Key::Unknown(0),
			shift: false,
			ctrl: false,
			alt: false,
			meta: false,
		}
	}
}
impl HotKey {
	pub fn relevant_keys(&self) -> HashSet<rdev::Key> {
		let mut keys = HashSet::with_capacity(9);
		keys.insert(self.code);
		if self.shift {
			keys.insert(rdev::Key::ShiftLeft);
			keys.insert(rdev::Key::ShiftRight);
		}
		if self.ctrl {
			keys.insert(rdev::Key::ControlLeft);
			keys.insert(rdev::Key::ControlRight);
		}
		if self.alt {
			keys.insert(rdev::Key::Alt);
			keys.insert(rdev::Key::AltGr);
		}
		if self.meta {
			keys.insert(rdev::Key::MetaLeft);
			keys.insert(rdev::Key::MetaRight);
		}
		keys
	}

	fn insert(&mut self, code: rdev::Key) {
		match code {
			rdev::Key::ShiftLeft | rdev::Key::ShiftRight => self.shift = true,
			rdev::Key::ControlLeft | rdev::Key::ControlRight => self.ctrl = true,
			rdev::Key::Alt | rdev::Key::AltGr => self.alt = true,
			rdev::Key::MetaLeft | rdev::Key::MetaRight => self.meta = true,
			_ => self.code = code,
		}
	}

	fn is_missing_mod(
		code: rdev::Key,
		want_mod: bool,
		mod_types: &[rdev::Key],
		pressed_keys: &HashSet<rdev::Key>,
	) -> bool {
		let any_mod_pressed = mod_types
			.iter()
			.fold(false, |any_pressed, key| any_pressed || pressed_keys.contains(key));
		!mod_types.contains(&code) && want_mod != any_mod_pressed
	}

	pub fn is_pressed(&self, keys: &HashSet<rdev::Key>) -> bool {
		if !keys.contains(&self.code) {
			return false;
		}

		if Self::is_missing_mod(
			self.code,
			self.shift,
			&[rdev::Key::ShiftLeft, rdev::Key::ShiftRight],
			keys,
		) {
			return false;
		}

		if Self::is_missing_mod(
			self.code,
			self.ctrl,
			&[rdev::Key::ControlLeft, rdev::Key::ControlRight],
			keys,
		) {
			return false;
		}

		if Self::is_missing_mod(self.code, self.alt, &[rdev::Key::Alt, rdev::Key::AltGr], keys) {
			return false;
		}

		if Self::is_missing_mod(self.code, self.meta, &[rdev::Key::MetaLeft, rdev::Key::MetaRight], keys) {
			return false;
		}

		true
	}
}
impl std::fmt::Display for HotKey {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}", self.code)?;
		if self.shift {
			write!(f, "+shift")?;
		}
		if self.ctrl {
			write!(f, "+ctrl")?;
		}
		if self.alt {
			write!(f, "+alt")?;
		}
		if self.meta {
			write!(f, "+meta")?;
		}
		Ok(())
	}
}

pub fn alias_hotkeys(combo: &shared::KeyCombo) -> Vec<HotKey> {
	let mut hotkeys = Vec::with_capacity(3);
	
	if let Some(alias) = combo.get_single() {
		// Simple conversions, alias directly matches some code
		if let Some(code) = key_alias_to_code(alias) {
			hotkeys.push(HotKey {
				code,
				..Default::default()
			});
			// Lower to Upper casings
			if alias.is_alpha() {
				hotkeys.push(HotKey {
					code,
					shift: true,
					..Default::default()
				});
			}
		}
	
		// Symbols which are represented by other codes
		if let Some(code) = dealias_code(alias) {
			hotkeys.push(HotKey {
				code,
				shift: true,
				..Default::default()
			});
		}
	}
	else {
		let mut hotkey = HotKey::default();
		for alias in combo.iter() {
			let Some(code) = key_alias_to_code(*alias) else { continue };
			hotkey.insert(code);
		}
		hotkeys.push(hotkey);
	}

	hotkeys
}
