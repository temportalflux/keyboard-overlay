use kdlize::AsKdl;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Switch {
	pub pos: (f32, f32),
	pub side: Option<Side>,
	pub os_key: OSKey,
}

impl kdlize::FromKdl<()> for Switch {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut kdlize::NodeReader<'doc, ()>) -> Result<Self, Self::Error> {
		let os_key = node.next_str_req_t::<OSKey>()?;
		let x = node.next_f64_req()? as f32;
		let y = node.next_f64_req()? as f32;
		let side = node.get_str_opt_t::<Side>("side")?;
		Ok(Self { pos: (x, y), side, os_key })
	}
}

impl AsKdl for Switch {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		node.push_entry(self.pos.0 as f64);
		node.push_entry(self.pos.1 as f64);
		if let Some(side) = self.side {
			node.push_entry(("side", side.to_string()));
		}
		node
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Side {
	Left,
	Right,
}
impl std::fmt::Display for Side {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Side::Left => "left",
				Side::Right => "right",
			}
		)
	}
}

impl std::str::FromStr for Side {
	type Err = InvalidSide;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"left" => Ok(Self::Left),
			"right" => Ok(Self::Right),
			_ => Err(InvalidSide(s.to_owned())),
		}
	}
}

#[derive(thiserror::Error, Debug)]
#[error("Invalid Side {0}, expecting \"left\" or \"right\"")]
pub struct InvalidSide(String);

#[derive(thiserror::Error, Debug)]
#[error("Invalid key id {0}")]
pub struct InvalidOSKey(String);

/// Literal USB key id that the os interprets based on provided modifiers.
/// See [this for more](https://www.reddit.com/r/ErgoMechKeyboards/comments/ujhp0g/comment/i7j0nko/?utm_source=share&utm_medium=web3x&utm_name=web3xcss&utm_term=1&utm_content=share_button).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OSKey {
	F1,
	F2,
	F3,
	F4,
	F5,
	F6,
	F7,
	F8,
	F9,
	F10,
	F11,
	F12,

	Function,
	/// Alt key on Linux and Windows (option key on macOS)
	AltLeft,
	AltRight,
	ControlLeft,
	ControlRight,
	ShiftLeft,
	ShiftRight,
	/// also known as "windows", "super", and "command"
	MetaLeft,
	/// also known as "windows", "super", and "command"
	MetaRight,

	CapsLock,
	NumLock,
	ScrollLock,
	
	Tab,
	Space,
	Backspace,
	Delete,
	Return,
	Escape,

	LeftArrow,
	DownArrow,
	RightArrow,
	UpArrow,
	
	Insert,
	Home,
	End,
	PageUp,
	PageDown,
	PrintScreen,
	Pause,

	Num1,
	Num2,
	Num3,
	Num4,
	Num5,
	Num6,
	Num7,
	Num8,
	Num9,
	Num0,

	Minus,
	Equal,
	SemiColon,
	Comma,
	Dot,
	Quote,
	Grave,
	Slash,
	BackSlash,
	IntlBackslash,
	BracketLeft,
	BracketRight,

	KeyQ,
	KeyW,
	KeyE,
	KeyR,
	KeyT,
	KeyY,
	KeyU,
	KeyI,
	KeyO,
	KeyP,
	KeyA,
	KeyS,
	KeyD,
	KeyF,
	KeyG,
	KeyH,
	KeyJ,
	KeyK,
	KeyL,
	KeyZ,
	KeyX,
	KeyC,
	KeyV,
	KeyB,
	KeyN,
	KeyM,

	Keypad0,
	Keypad1,
	Keypad2,
	Keypad3,
	Keypad4,
	Keypad5,
	Keypad6,
	Keypad7,
	Keypad8,
	Keypad9,
	KeypadMinus,
	KeypadPlus,
	KeypadMultiply,
	KeypadDivide,
	KeypadReturn,
	KeypadDelete,

	Unknown(u32),
}

impl std::fmt::Display for OSKey {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", match self {
			Self::F1 => "F1",
			Self::F2 => "F2",
			Self::F3 => "F3",
			Self::F4 => "F4",
			Self::F5 => "F5",
			Self::F6 => "F6",
			Self::F7 => "F7",
			Self::F8 => "F8",
			Self::F9 => "F9",
			Self::F10 => "F10",
			Self::F11 => "F11",
			Self::F12 => "F12",
			Self::Function => "Function",
			Self::AltLeft => "AltLeft",
			Self::AltRight => "AltRight",
			Self::ControlLeft => "ControlLeft",
			Self::ControlRight => "ControlRight",
			Self::ShiftLeft => "ShiftLeft",
			Self::ShiftRight => "ShiftRight",
			Self::MetaLeft => "MetaLeft",
			Self::MetaRight => "MetaRight",
			Self::CapsLock => "CapsLock",
			Self::NumLock => "NumLock",
			Self::ScrollLock => "ScrollLock",
			Self::Tab => "Tab",
			Self::Space => "Space",
			Self::Backspace => "Backspace",
			Self::Delete => "Delete",
			Self::Return => "Return",
			Self::Escape => "Escape",
			Self::LeftArrow => "LeftArrow",
			Self::DownArrow => "DownArrow",
			Self::RightArrow => "RightArrow",
			Self::UpArrow => "UpArrow",
			Self::Insert => "Insert",
			Self::Home => "Home",
			Self::End => "End",
			Self::PageUp => "PageUp",
			Self::PageDown => "PageDown",
			Self::PrintScreen => "PrintScreen",
			Self::Pause => "Pause",
			Self::Num1 => "1",
			Self::Num2 => "2",
			Self::Num3 => "3",
			Self::Num4 => "4",
			Self::Num5 => "5",
			Self::Num6 => "6",
			Self::Num7 => "7",
			Self::Num8 => "8",
			Self::Num9 => "9",
			Self::Num0 => "0",
			Self::Minus => "Minus",
			Self::Equal => "Equal",
			Self::SemiColon => "SemiColon",
			Self::Comma => "Comma",
			Self::Dot => "Dot",
			Self::Quote => "Quote",
			Self::Grave => "Grave",
			Self::Slash => "Slash",
			Self::BackSlash => "BackSlash",
			Self::IntlBackslash => "IntlBackslash",
			Self::BracketLeft => "BracketLeft",
			Self::BracketRight => "BracketRight",
			Self::KeyQ => "Q",
			Self::KeyW => "W",
			Self::KeyE => "E",
			Self::KeyR => "R",
			Self::KeyT => "T",
			Self::KeyY => "Y",
			Self::KeyU => "U",
			Self::KeyI => "I",
			Self::KeyO => "O",
			Self::KeyP => "P",
			Self::KeyA => "A",
			Self::KeyS => "S",
			Self::KeyD => "D",
			Self::KeyF => "F",
			Self::KeyG => "G",
			Self::KeyH => "H",
			Self::KeyJ => "J",
			Self::KeyK => "K",
			Self::KeyL => "L",
			Self::KeyZ => "Z",
			Self::KeyX => "X",
			Self::KeyC => "C",
			Self::KeyV => "V",
			Self::KeyB => "B",
			Self::KeyN => "N",
			Self::KeyM => "M",
			Self::Keypad0 => "Keypad0",
			Self::Keypad1 => "Keypad1",
			Self::Keypad2 => "Keypad2",
			Self::Keypad3 => "Keypad3",
			Self::Keypad4 => "Keypad4",
			Self::Keypad5 => "Keypad5",
			Self::Keypad6 => "Keypad6",
			Self::Keypad7 => "Keypad7",
			Self::Keypad8 => "Keypad8",
			Self::Keypad9 => "Keypad9",
			Self::KeypadMinus => "KeypadMinus",
			Self::KeypadPlus => "KeypadPlus",
			Self::KeypadMultiply => "KeypadMultiply",
			Self::KeypadDivide => "KeypadDivide",
			Self::KeypadReturn => "KeypadReturn",
			Self::KeypadDelete => "KeypadDelete",
			Self::Unknown(value) => {
				return write!(f, "u{value}");
			},
		})	
	}
}

impl std::str::FromStr for OSKey {
	type Err = InvalidOSKey;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"F1" => Ok(Self::F1),
			"F2" => Ok(Self::F2),
			"F3" => Ok(Self::F3),
			"F4" => Ok(Self::F4),
			"F5" => Ok(Self::F5),
			"F6" => Ok(Self::F6),
			"F7" => Ok(Self::F7),
			"F8" => Ok(Self::F8),
			"F9" => Ok(Self::F9),
			"F10" => Ok(Self::F10),
			"F11" => Ok(Self::F11),
			"F12" => Ok(Self::F12),
			"Function" => Ok(Self::Function),
			"AltLeft" => Ok(Self::AltLeft),
			"AltRight" => Ok(Self::AltRight),
			"ControlLeft" => Ok(Self::ControlLeft),
			"ControlRight" => Ok(Self::ControlRight),
			"ShiftLeft" => Ok(Self::ShiftLeft),
			"ShiftRight" => Ok(Self::ShiftRight),
			"MetaLeft" => Ok(Self::MetaLeft),
			"MetaRight" => Ok(Self::MetaRight),
			"CapsLock" => Ok(Self::CapsLock),
			"NumLock" => Ok(Self::NumLock),
			"ScrollLock" => Ok(Self::ScrollLock),
			"Tab" => Ok(Self::Tab),
			"Space" => Ok(Self::Space),
			"Backspace" => Ok(Self::Backspace),
			"Delete" => Ok(Self::Delete),
			"Return" => Ok(Self::Return),
			"Escape" => Ok(Self::Escape),
			"LeftArrow" => Ok(Self::LeftArrow),
			"DownArrow" => Ok(Self::DownArrow),
			"RightArrow" => Ok(Self::RightArrow),
			"UpArrow" => Ok(Self::UpArrow),
			"Insert" => Ok(Self::Insert),
			"Home" => Ok(Self::Home),
			"End" => Ok(Self::End),
			"PageUp" => Ok(Self::PageUp),
			"PageDown" => Ok(Self::PageDown),
			"PrintScreen" => Ok(Self::PrintScreen),
			"Pause" => Ok(Self::Pause),
			"1" => Ok(Self::Num1),
			"2" => Ok(Self::Num2),
			"3" => Ok(Self::Num3),
			"4" => Ok(Self::Num4),
			"5" => Ok(Self::Num5),
			"6" => Ok(Self::Num6),
			"7" => Ok(Self::Num7),
			"8" => Ok(Self::Num8),
			"9" => Ok(Self::Num9),
			"0" => Ok(Self::Num0),
			"Minus" => Ok(Self::Minus),
			"Equal" => Ok(Self::Equal),
			"SemiColon" => Ok(Self::SemiColon),
			"Comma" => Ok(Self::Comma),
			"Dot" => Ok(Self::Dot),
			"Quote" => Ok(Self::Quote),
			"Grave" => Ok(Self::Grave),
			"Slash" => Ok(Self::Slash),
			"BackSlash" => Ok(Self::BackSlash),
			"IntlBackslash" => Ok(Self::IntlBackslash),
			"BracketLeft" => Ok(Self::BracketLeft),
			"BracketRight" => Ok(Self::BracketRight),
			"Q" => Ok(Self::KeyQ),
			"W" => Ok(Self::KeyW),
			"E" => Ok(Self::KeyE),
			"R" => Ok(Self::KeyR),
			"T" => Ok(Self::KeyT),
			"Y" => Ok(Self::KeyY),
			"U" => Ok(Self::KeyU),
			"I" => Ok(Self::KeyI),
			"O" => Ok(Self::KeyO),
			"P" => Ok(Self::KeyP),
			"A" => Ok(Self::KeyA),
			"S" => Ok(Self::KeyS),
			"D" => Ok(Self::KeyD),
			"F" => Ok(Self::KeyF),
			"G" => Ok(Self::KeyG),
			"H" => Ok(Self::KeyH),
			"J" => Ok(Self::KeyJ),
			"K" => Ok(Self::KeyK),
			"L" => Ok(Self::KeyL),
			"Z" => Ok(Self::KeyZ),
			"X" => Ok(Self::KeyX),
			"C" => Ok(Self::KeyC),
			"V" => Ok(Self::KeyV),
			"B" => Ok(Self::KeyB),
			"N" => Ok(Self::KeyN),
			"M" => Ok(Self::KeyM),
			"Keypad0" => Ok(Self::Keypad0),
			"Keypad1" => Ok(Self::Keypad1),
			"Keypad2" => Ok(Self::Keypad2),
			"Keypad3" => Ok(Self::Keypad3),
			"Keypad4" => Ok(Self::Keypad4),
			"Keypad5" => Ok(Self::Keypad5),
			"Keypad6" => Ok(Self::Keypad6),
			"Keypad7" => Ok(Self::Keypad7),
			"Keypad8" => Ok(Self::Keypad8),
			"Keypad9" => Ok(Self::Keypad9),
			"KeypadMinus" => Ok(Self::KeypadMinus),
			"KeypadPlus" => Ok(Self::KeypadPlus),
			"KeypadMultiply" => Ok(Self::KeypadMultiply),
			"KeypadDivide" => Ok(Self::KeypadDivide),
			"KeypadReturn" => Ok(Self::KeypadReturn),
			"KeypadDelete" => Ok(Self::KeypadDelete),
			// Aliases
			"!" => Ok(Self::Num1),
			"@" => Ok(Self::Num2),
			"#" => Ok(Self::Num3),
			"$" => Ok(Self::Num4),
			"%" => Ok(Self::Num5),
			"^" => Ok(Self::Num6),
			"&" => Ok(Self::Num7),
			"*" => Ok(Self::Num8),
			"ParenLeft" => Ok(Self::Num9),
			"(" => Ok(Self::Num9),
			"ParenRight" => Ok(Self::Num0),
			")" => Ok(Self::Num0),
			"-" => Ok(Self::Minus),
			"_" => Ok(Self::Minus),
			"=" => Ok(Self::Equal),
			"+" => Ok(Self::Equal),
			"~" => Ok(Self::Grave),
			"`" => Ok(Self::Grave),
			";" => Ok(Self::SemiColon),
			":" => Ok(Self::SemiColon),
			"\"" => Ok(Self::Quote),
			"'" => Ok(Self::Quote),
			"," => Ok(Self::Comma),
			"<" => Ok(Self::Comma),
			"." => Ok(Self::Dot),
			">" => Ok(Self::Dot),
			"/" => Ok(Self::Slash),
			"?" => Ok(Self::Slash),
			"[" => Ok(Self::BracketLeft),
			"{" => Ok(Self::BracketLeft),
			"]" => Ok(Self::BracketRight),
			"}" => Ok(Self::BracketRight),
			"\\" => Ok(Self::BackSlash),
			"|" => Ok(Self::BackSlash),
			// Unknown/fallback
			s =>  {
				let Some(value_str) = s.strip_prefix("u") else {
					return Err(InvalidOSKey(s.to_owned()));
				};
				let value = value_str.parse::<u32>().map_err(|_| InvalidOSKey(s.to_owned()))?;
				Ok(Self::Unknown(value))
			}
		}
	}
}
