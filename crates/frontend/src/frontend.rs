use futures::{SinkExt, StreamExt};
use shared::{Binding, BindingDisplay, BoundSwitch, InputUpdate, Layout, Side, SwitchSlot};
use std::collections::{BTreeMap, HashSet};
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

#[derive(Clone, Debug, Default, PartialEq)]
struct InputState {
	active_layers: HashSet<String>,
	active_switches: BTreeMap<String, (SwitchSlot, wasm_timer::Instant)>,
}

#[function_component]
fn App() -> Html {
	let icon_scale = use_state_eq(|| 1.0f64);
	let layout = use_state_eq(|| None::<Layout>);
	let input_state = use_state_eq(|| InputState::default());

	let icon_scale_handle = icon_scale.clone();
	let layout_handle = layout.clone();
	let input_handle = input_state.clone();
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

		let (send_input, mut recv_input) = futures::channel::mpsc::unbounded::<InputUpdate>();

		spawn_local("input::recv", {
			let mut send_input = send_input.clone();
			async move {
				let mut stream = listen::<InputUpdate>("input").await?;
				while let Some(event) = stream.next().await {
					log::debug!(target: "recv::input", "update: {:?}", event.payload);
					send_input.send(event.payload).await?;
				}
				Ok(()) as anyhow::Result<()>
			}
		});

		let input_state = input_handle.clone();
		spawn_local("input::process", async move {
			static MIN_PRESS_DURATION: std::time::Duration = std::time::Duration::from_millis(100);
			let mut local_state = InputState::default();
			while let Some(update) = recv_input.next().await {
				match update {
					InputUpdate::LayerActivate(layer) => {
						local_state.active_layers.insert(layer);
					}
					InputUpdate::LayerDeactivate(layer) => {
						local_state.active_layers.remove(&layer);
					}
					InputUpdate::SwitchPressed(switch_id, slot) => {
						local_state
							.active_switches
							.insert(switch_id, (slot, wasm_timer::Instant::now()));
					}
					InputUpdate::SwitchReleased(switch_id) => {
						let latent_remove_duration = match local_state.active_switches.get(&switch_id) {
							None => continue,
							Some((_slot, start_time)) => {
								let now = wasm_timer::Instant::now();
								let duration_since_pressed = now.duration_since(*start_time);
								let duration_remaining = MIN_PRESS_DURATION.saturating_sub(duration_since_pressed);
								(!duration_remaining.is_zero()).then_some(duration_remaining)
							}
						};
	
						match latent_remove_duration {
							None => {
								local_state.active_switches.remove(&switch_id);
							}
							Some(duration_remaining) => {
								let mut send_input = send_input.clone();
								spawn_local("recv::input::latent_release", async move {
									gloo_timers::future::TimeoutFuture::new(duration_remaining.as_millis() as u32).await;
									send_input.send(InputUpdate::SwitchReleased(switch_id)).await?;
									Ok(()) as anyhow::Result<()>
								});
								continue;
							}
						}
					}
				}
				input_state.set(local_state.clone());
			}
			Ok(()) as anyhow::Result<()>
		});

		spawn_local("ready", tauri_sys::event::emit("ready", &()));
	});

	let layout_style = Style::default().with("--icon-scale", *icon_scale);
	log::debug!("{:?}", *input_state);

	let mut switches = Vec::with_capacity(40);
	if let Some(layout) = layout.as_ref() {
		'switch: for (switch_id, switch) in layout.switches().iter() {
			for layer_id in layout.layer_order().iter().rev() {
				if !input_state.active_layers.contains(layer_id) {
					continue;
				}
				let Some(layer) = layout.get_layer(layer_id) else {
					continue;
				};
				let Some(bindings) = layer.get_binding(switch_id) else {
					continue;
				};
				let active_slot = input_state.active_switches.get(switch_id);
				let active_slot = active_slot.map(|(slot, _start_time)| slot);

				switches.push(html!(<KeySwitch
					switch_id={switch_id.clone()}
					switch={*switch}
					bindings={bindings.clone()}
					active_slot={active_slot.cloned()}
				/>));

				continue 'switch;
			}
		}
	}

	html! {<>
		<div class="guideline x" />
		<div class="guideline y" />
		<div style="display: none;"><img src="https://raw.githubusercontent.com/tapioki/cephalopoda/main/Images/architeuthis_dux.png" style="height: 400px; margin-left: -150px; margin-top: 100px;" /></div>
		<div style={layout_style}>
			{switches}
		</div>
	</>}
}

#[derive(Clone, PartialEq, Properties)]
pub struct KeySwitchProps {
	pub switch_id: AttrValue,
	pub switch: shared::Switch,
	pub bindings: BoundSwitch,
	pub active_slot: Option<SwitchSlot>,
}

#[function_component]
fn KeySwitch(
	KeySwitchProps {
		switch_id,
		switch,
		bindings,
		active_slot,
	}: &KeySwitchProps,
) -> Html {
	let mut class = classes!("switch");
	let mut pos = switch.pos;
	if switch.side.is_some() {
		pos.0 = pos.0.abs();
	}
	let style = Style::from([("--x", format!("{}px", pos.0)), ("--y", format!("{}px", pos.1))]);

	if active_slot.is_some() {
		class.push("active");
	}

	fn element(slot: &SwitchSlot, binding: &Binding) -> Html {
		let mut class = classes!("slot");
		match slot {
			SwitchSlot::Tap => class.push("center"),
			SwitchSlot::Hold => class.push("bottom"),
		}
		let element = match &binding.display {
			None => html!(<div class="label">{binding.input.to_string()}</div>),
			Some(BindingDisplay::Text(value)) => html!(<div class="label">{value}</div>),
			Some(BindingDisplay::IconBootstrap(value)) => html!(
				<i class={format!("bi bi-{value}")} />
			),
			Some(BindingDisplay::IconCustom(value)) => html!(
				<img class={"icon"} style={format!("--glyph: url(assets/glyph/{value}.svg);")} />
			),
		};

		let layer = binding.layer.clone();
		html!(<div {class} {layer}>{element}</div>)
	}

	let mut contents = Vec::new();
	for (slot, binding) in &bindings.slots {
		contents.push(element(slot, binding));
	}

	let side = switch.side.as_ref().map(Side::to_string);
	let active_slot = active_slot.as_ref().map(SwitchSlot::to_string);
	html!(<div id={switch_id.clone()} {class} {style} {side} {active_slot}>
		{contents}
	</div>)
}
