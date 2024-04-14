use futures::StreamExt;
use shared::{InputUpdate, Layout, Side};
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

fn sample_layout() -> anyhow::Result<Layout> {
	static LOCAL_CONFIG: &'static str = include_str!("../../../config.kdl");
	let config_doc = LOCAL_CONFIG.parse::<kdl::KdlDocument>()?;
	let mut doc_node = kdl::KdlNode::new("document");
	doc_node.set_children(config_doc);
	let node = kdlize::NodeReader::new_root(&doc_node, ());
	let layout = node.query_req_t("scope() > layout")?;
	Ok(layout)
}

#[function_component]
fn App() -> Html {
	let icon_scale = use_state_eq(|| 1.0f64);
	let layout = use_state_eq(|| None::<Layout>);
	let input_update = use_state_eq(|| None::<InputUpdate>);

	let icon_scale_handle = icon_scale.clone();
	let layout_handle = layout.clone();
	let input_handle = input_update.clone();
	use_mount(move || {
		if !is_bound() {
			log::debug!("ignoring event listeners");
			layout_handle.set(sample_layout().ok());
			return;
		}
		log::debug!("mounting event listeners");

		let icon_scale = icon_scale_handle.clone();
		spawn_local("recv::scale", async move {
			let mut stream = listen::<f64>("scale").await?;
			while let Some(event) = stream.next().await {
				icon_scale.set(event.payload);
			}
			Ok(()) as anyhow::Result<()>
		});

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

	let layout_style = Style::default().with("--icon-scale", *icon_scale);

	html! {<>
		<div class="guideline x" />
		<div class="guideline y" />
		<div style="display: none;"><img src="https://raw.githubusercontent.com/tapioki/cephalopoda/main/Images/architeuthis_dux.png" style="height: 400px; margin-left: -150px; margin-top: 100px;" /></div>
		<div style={layout_style}>
			{layout.as_ref().map(|layout| {
				let layer = layout.get_layer(layout.default_layer())?;
				let iter = layout.switches().iter();
				let iter = iter.map(|(switch_id, switch)| (switch_id, switch, layer.get_binding(switch_id)));
				let switches = iter.map(|(switch_id, switch, binding)| html!(
					<KeySwitch
						switch_id={switch_id.clone()}
						switch={*switch}
						binding={binding.cloned()}
						is_active={input_update.as_ref().map(|input| input.0.contains(switch_id)).unwrap_or(false)}
					/>
				)).collect::<Vec<_>>();
				Some(html!(<>{switches}</>))
			}).flatten()}
		</div>
	</>}
}

#[derive(Clone, PartialEq, Properties)]
pub struct KeySwitchProps {
	pub switch_id: AttrValue,
	pub switch: shared::Switch,
	pub binding: Option<String>,
	pub is_active: bool,
}

#[function_component]
fn KeySwitch(
	KeySwitchProps {
		switch_id,
		switch,
		binding,
		is_active,
	}: &KeySwitchProps,
) -> Html {
	let class = classes!("key");
	let mut pos = switch.pos;
	if switch.side.is_some() {
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
	html!(<div id={switch_id.clone()} {class} {style} side={switch.side.as_ref().map(Side::to_string)}>
		<InputGlyph name={binding.clone()} style={glyph_style} />
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
	pub name: AttrValue,
	pub style: Option<InputGlyphStyle>,
}

#[function_component]
pub fn InputGlyph(InputGlyphProps { name, style }: &InputGlyphProps) -> Html {
	let mut class = classes!("input-glyph", name);
	let mut src = format!("assets/input-prompts");
	if let Some(style) = style {
		src += &format!("/{style}");
		class.push(style.to_string());
	}
	src += &format!("/{name}.svg");

	let style = Style::from([("--glyph", format!("url({src})"))]);
	html!(<img {class} {style} />)
}
