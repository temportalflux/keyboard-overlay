use futures::StreamExt;
use shared::{InputUpdate, Layout};
use tauri_sys::event::listen;
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew_hooks::use_mount;

mod style;
pub use style::*;
mod logging;
pub mod utility;
use utility::spawn_local;

#[wasm_bindgen(module = "/glue.js")]
extern "C" {
	#[wasm_bindgen(js_name = isBound)]
	pub fn is_bound() -> bool;
}

#[cfg(target_family = "wasm")]
fn main() {
	if is_bound() {
		let _ = ::log::set_boxed_logger(Box::new(logging::tauri::Logger));
		::log::set_max_level(log::LevelFilter::Trace);
	} else {
		use logging::wasm::*;
		init(Config::default().prefer_target());
	}
	yew::Renderer::<App>::new().render();
}

#[cfg(target_family = "windows")]
fn main() {}

#[derive(Clone, PartialEq)]
enum Side {
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

#[derive(Clone, PartialEq)]
pub struct LayoutKey {
	switch_id: String,

	pos: (f32, f32),
	side: Option<Side>,

	key_name: String,
	key_source: InputSource,
}

#[function_component]
fn App() -> Html {
	let layout = use_state_eq(|| None::<Layout>);
	let input_update = use_state_eq(|| None::<InputUpdate>);

	let layout_handle = layout.clone();
	let input_handle = input_update.clone();
	use_mount(move || {
		if !is_bound() {
			log::debug!("ignoring event listeners");
			return;
		}
		log::debug!("mounting event listeners");

		let layout = layout_handle.clone();
		spawn_local("recv::layout", async move {
			let mut stream = listen::<Layout>("layout").await?;
			while let Some(event) = stream.next().await {
				log::debug!(target: "recv::layout", "layout update: {:?}", event.payload);
				layout.set(Some(event.payload));
			}
			Ok(()) as anyhow::Result<()>
		});

		let input_update = input_handle.clone();
		spawn_local("recv::input", async move {
			let mut stream = listen::<InputUpdate>("input").await?;
			while let Some(event) = stream.next().await {
				log::debug!(target: "recv::input", "update: {:?}", event.payload);
				input_update.set(Some(event.payload));
			}
			Ok(()) as anyhow::Result<()>
		});

		spawn_local("ready", tauri_sys::event::emit("ready", &()));
	});

	let layout_keys = vec![
		LayoutKey {
			switch_id: "l1".into(),
			pos: (100f32, 60f32),
			side: Some(Side::Left),
			key_name: "arrow_up".into(),
			key_source: InputSource::Keyboard,
		},
		LayoutKey {
			switch_id: "l2".into(),
			pos: (100f32, 0f32),
			side: Some(Side::Left),
			key_name: "arrow_down".into(),
			key_source: InputSource::Keyboard,
		},
		LayoutKey {
			switch_id: "l3".into(),
			pos: (160f32, 0f32),
			side: Some(Side::Left),
			key_name: "arrow_left".into(),
			key_source: InputSource::Keyboard,
		},
		LayoutKey {
			switch_id: "l4".into(),
			pos: (40f32, 0f32),
			side: Some(Side::Left),
			key_name: "arrow_right".into(),
			key_source: InputSource::Keyboard,
		},
		LayoutKey {
			switch_id: "r1".into(),
			pos: (100f32, 60f32),
			side: Some(Side::Right),
			key_name: "arrow_up".into(),
			key_source: InputSource::Keyboard,
		},
		LayoutKey {
			switch_id: "r2".into(),
			pos: (100f32, 0f32),
			side: Some(Side::Right),
			key_name: "arrow_down".into(),
			key_source: InputSource::Keyboard,
		},
		LayoutKey {
			switch_id: "r3".into(),
			pos: (40f32, 0f32),
			side: Some(Side::Right),
			key_name: "arrow_left".into(),
			key_source: InputSource::Keyboard,
		},
		LayoutKey {
			switch_id: "r4".into(),
			pos: (160f32, 0f32),
			side: Some(Side::Right),
			key_name: "arrow_right".into(),
			key_source: InputSource::Keyboard,
		},
	];

	html! {<>
		<div>{format!("{:?}", *layout)}</div>
		{layout_keys.into_iter().map(|key| html!(<KeySwitch is_active={key.side == Some(Side::Left)} binding={key} />)).collect::<Vec<_>>()}
	</>}
}

#[derive(Clone, PartialEq, Properties)]
pub struct KeySwitchProps {
	pub binding: LayoutKey,
	pub is_active: bool,
}

#[function_component]
fn KeySwitch(
	KeySwitchProps {
		binding: key,
		is_active,
	}: &KeySwitchProps,
) -> Html {
	let class = classes!("key");
	let mut pos = key.pos;
	if key.side.is_some() {
		pos.0 = pos.0.abs();
	}
	let style = Style::from([("--x", format!("{}px", pos.0)), ("--y", format!("{}px", pos.1))]);
	let glyph_style = match is_active {
		false => InputGlyphStyle::Outline,
		true => InputGlyphStyle::Fill,
	};
	html!(<div id={key.switch_id.clone()} {class} {style} side={key.side.as_ref().map(Side::to_string)}>
		<InputGlyph name={key.key_name.clone()} source={key.key_source} style={glyph_style} />
	</div>)
}

#[derive(Clone, Copy, PartialEq)]
pub enum InputSource {
	Keyboard,
	Mouse,
}
impl std::fmt::Display for InputSource {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::Keyboard => "keyboard",
				Self::Mouse => "mouse",
			}
		)
	}
}

#[derive(Clone, Copy, PartialEq)]
pub enum InputGlyphStyle {
	Fill,
	Outline,
}
impl std::fmt::Display for InputGlyphStyle {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::Fill => "fill",
				Self::Outline => "outline",
			}
		)
	}
}

#[derive(Clone, PartialEq, Properties)]
pub struct InputGlyphProps {
	pub source: InputSource,
	pub name: AttrValue,
	pub style: Option<InputGlyphStyle>,
}

#[function_component]
pub fn InputGlyph(InputGlyphProps { source, name, style }: &InputGlyphProps) -> Html {
	let mut class = classes!("input-glyph", source.to_string(), name);
	let mut src = format!("assets/input-prompts/{source}");
	if let Some(style) = style {
		src += &format!("/{style}");
		class.push(style.to_string());
	}
	src += &format!("/{name}.svg");

	let style = Style::from([("--glyph", format!("url({src})"))]);
	html!(<img {class} {style} />)
}
