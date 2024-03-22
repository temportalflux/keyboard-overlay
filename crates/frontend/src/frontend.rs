use futures::StreamExt;
use shared::{InputSource, InputUpdate, Layout, Side};
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
				//log::debug!(target: "recv::layout", "layout update: {:?}", event.payload);
				layout.set(Some(event.payload));
			}
			Ok(()) as anyhow::Result<()>
		});

		let input_update = input_handle.clone();
		spawn_local("recv::input", async move {
			let mut stream = listen::<InputUpdate>("input").await?;
			while let Some(event) = stream.next().await {
				//log::debug!(target: "recv::input", "update: {:?}", event.payload);
				input_update.set(Some(event.payload));
			}
			Ok(()) as anyhow::Result<()>
		});

		spawn_local("ready", tauri_sys::event::emit("ready", &()));
	});

	html! {<>
		<div class="guideline x" />
		<div class="guideline y" />
		<div style="display: none;"><img src="https://raw.githubusercontent.com/tapioki/cephalopoda/main/Images/architeuthis_dux.png" style="height: 400px; margin-left: -150px; margin-top: 100px;" /></div>
		{layout.as_ref().map(|layout| {
			let layer = layout.get_layer(layout.default_layer())?;
			let iter = layout.switches().iter();
			let iter = iter.map(|(switch, location)| (switch, location, layer.get_binding(switch)));
			let switches = iter.map(|(switch, location, binding)| html!(
				<KeySwitch
					switch={switch.clone()}
					location={*location}
					binding={binding.cloned()}
					is_active={input_update.as_ref().map(|input| input.0.contains(switch)).unwrap_or(false)}
				/>
			)).collect::<Vec<_>>();
			Some(html!(<>{switches}</>))
		}).flatten()}
	</>}
}

#[derive(Clone, PartialEq, Properties)]
pub struct KeySwitchProps {
	pub switch: AttrValue,
	pub location: shared::SwitchLocation,
	pub binding: Option<shared::KeyBinding>,
	pub is_active: bool,
}

#[function_component]
fn KeySwitch(
	KeySwitchProps {
		switch,
		location,
		binding,
		is_active,
	}: &KeySwitchProps,
) -> Html {
	let class = classes!("key");
	let mut pos = location.pos;
	if location.side.is_some() {
		pos.0 = pos.0.abs();
	}
	let style = Style::from([
		("--x", format!("{}px", pos.0)),
		("--y", format!("{}px", pos.1)),
		("--width", "64px".into()),
		("--height", "64px".into()),
	]);
	let glyph_style = match is_active {
		false => InputGlyphStyle::Outline,
		true => InputGlyphStyle::Fill,
	};
	let binding = binding.clone().unwrap_or_default();
	html!(<div id={switch.clone()} {class} {style} side={location.side.as_ref().map(Side::to_string)}>
		<InputGlyph name={binding.key.clone()} source={binding.source} style={glyph_style} />
	</div>)
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
