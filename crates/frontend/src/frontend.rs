use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::window;
use yew::prelude::*;

mod style;
pub use style::*;

#[wasm_bindgen(module = "/glue.js")]
extern "C" {
	#[wasm_bindgen(js_name = isBound)]
	pub fn is_bound() -> bool;

	#[wasm_bindgen(js_name = invokeHello, catch)]
	pub async fn hello(name: String) -> Result<JsValue, JsValue>;
}

#[cfg(target_family = "wasm")]
fn main() {
	yew::Renderer::<App>::new().render();
}

#[cfg(target_family = "windows")]
fn main() {}

#[function_component]
fn App() -> Html {
	let welcome = use_state_eq(|| "".to_string());
	let name = use_state_eq(|| "World".to_string());

	// Execute tauri command via effects.
	// The effect will run every time `name` changes.
	{
		let welcome = welcome.clone();
		use_effect_with((*name).clone(), move |name| {
			update_welcome_message(welcome, name.clone());
			|| ()
		});
	}

	let message = (*welcome).clone();

	html! {<>
		<h2 class={"heading"}>{message}</h2>

		<div class="key" style={Style::from([("--x", "100px"), ("--y", "60px"), ("--scale-x", "-1")])}>
			<InputGlyph name="arrow_up" source={InputSource::Keyboard} style={InputGlyphStyle::Outline} />
		</div>
		<div class="key" style={Style::from([("--x", "100px"), ("--y", "0px"), ("--scale-x", "-1")])}>
			<InputGlyph name="arrow_down" source={InputSource::Keyboard} style={InputGlyphStyle::Outline} />
		</div>
		<div class="key" style={Style::from([("--x", "160px"), ("--y", "0px"), ("--scale-x", "-1")])}>
			<InputGlyph name="arrow_left" source={InputSource::Keyboard} style={InputGlyphStyle::Outline} />
		</div>
		<div class="key" style={Style::from([("--x", "40px"), ("--y", "0px"), ("--scale-x", "-1")])}>
			<InputGlyph name="arrow_right" source={InputSource::Keyboard} style={InputGlyphStyle::Outline} />
		</div>

		<div class="key" style={Style::from([("--x", "100px"), ("--y", "60px"), ("--scale-x", "1")])}>
			<InputGlyph name="arrow_up" source={InputSource::Keyboard} style={InputGlyphStyle::Fill} />
		</div>
		<div class="key" style={Style::from([("--x", "100px"), ("--y", "0px"), ("--scale-x", "1")])}>
			<InputGlyph name="arrow_down" source={InputSource::Keyboard} style={InputGlyphStyle::Fill} />
		</div>
		<div class="key" style={Style::from([("--x", "40px"), ("--y", "0px"), ("--scale-x", "1")])}>
			<InputGlyph name="arrow_left" source={InputSource::Keyboard} style={InputGlyphStyle::Fill} />
		</div>
		<div class="key" style={Style::from([("--x", "160px"), ("--y", "0px"), ("--scale-x", "1")])}>
			<InputGlyph name="arrow_right" source={InputSource::Keyboard} style={InputGlyphStyle::Fill} />
		</div>
	</>}
}

fn update_welcome_message(welcome: UseStateHandle<String>, name: String) {
	spawn_local(async move {
		if !is_bound() {
			return;
		}
		// This will call our glue code all the way through to the tauri
		// back-end command and return the `Result<String, String>` as
		// `Result<JsValue, JsValue>`.
		match hello(name).await {
			Ok(message) => {
				welcome.set(message.as_string().unwrap());
			}
			Err(e) => {
				let window = window().unwrap();
				window.alert_with_message(&format!("Error: {:?}", e)).unwrap();
			}
		}
	});
}

#[derive(Clone, Copy, PartialEq)]
pub enum InputSource {
	Keyboard,
	Mouse,
}
impl std::fmt::Display for InputSource {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", match self {
			Self::Keyboard => "keyboard",
			Self::Mouse => "mouse",
		})
	}
}

#[derive(Clone, Copy, PartialEq)]
pub enum InputGlyphStyle {
	Fill,
	Outline,
}
impl std::fmt::Display for InputGlyphStyle {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", match self {
			Self::Fill => "fill",
			Self::Outline => "outline",
		})
	}
}

#[derive(Clone, PartialEq, Properties)]
pub struct InputGlyphProps {
	#[prop_or_default]
	pub id: Option<AttrValue>,
	pub source: InputSource,
	pub name: AttrValue,
	pub style: Option<InputGlyphStyle>,
}

#[function_component]
pub fn InputGlyph(InputGlyphProps { id, source, name, style }: &InputGlyphProps) -> Html {
	let mut class = classes!("input-glyph", source.to_string(), name);
	let mut src = format!("assets/input-prompts/{source}");
	if let Some(style) = style {
		src += &format!("/{style}");
		class.push(style.to_string());
	}
	src += &format!("/{name}.svg");

	let style = Style::from([ ("--glyph", format!("url({src})")) ]);
	html!(<img {id} {class} {style} />)
}
