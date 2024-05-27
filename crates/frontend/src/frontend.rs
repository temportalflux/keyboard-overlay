use futures::{SinkExt, StreamExt};
use shared::{Binding, BoundSwitch, InputUpdate, Layout, SwitchSlot};
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
	active_switches: BTreeMap<String, (Option<SwitchSlot>, wasm_timer::Instant)>,
}

#[function_component]
fn App() -> Html {
	let window_size = use_state_eq(|| (0u32, 0u32));
	let icon_scale = use_state_eq(|| 1.0f64);
	let layout = use_state_eq(|| None::<Layout>);
	let input_state = use_state_eq(|| InputState::default());

	let window_size_handle = window_size.clone();
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

		let window_size = window_size_handle.clone();
		let icon_scale = icon_scale_handle.clone();
		spawn_local("recv::scale", async move {
			let physical_size = tauri_sys::window::current_window().inner_size().await?;
			window_size.set((physical_size.width(), physical_size.height()));

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
					//log::debug!(target: "recv::input", "update: {:?}", event.payload);
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
									gloo_timers::future::TimeoutFuture::new(duration_remaining.as_millis() as u32)
										.await;
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
	//log::debug!("{:?}", *input_state);

	let mut switches = Vec::with_capacity(40);
	let mut combos = Vec::with_capacity(10);
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
				let active_slot = active_slot.map(|(slot, _start_time)| slot.clone()).flatten();

				switches.push(html!(<KeySwitch
					window_size={*window_size}
					switch_id={switch_id.clone()}
					switch={*switch}
					bindings={bindings.clone()}
					active_slot={active_slot}
				/>));

				continue 'switch;
			}
		}
		'combo: for combo in layout.combos().iter() {
			// Filter out combos that are not on an active layer
			if !combo.layers.is_empty() {
				let on_active_layer = combo
					.layers
					.iter()
					.any(|layer| input_state.active_layers.contains(layer));
				if !on_active_layer {
					continue 'combo;
				}
			}

			let mut class = classes!("switch", "combo");
			let size = 30f64;
			let pos = (combo.pos.0 as f64, combo.pos.1 as f64);
			let pos = calculate_screen_pos(&*window_size, pos, size);
			let style = Style::from([
				("--x", format!("{}px", pos.0)),
				("--y", format!("{}px", pos.1)),
				("width", format!("{size}px")),
				("height", format!("{size}px")),
				("border-width", format!("{SWITCH_BORDER_WIDTH}px")),
			]);

			if input_state.active_switches.contains_key(&combo.id) {
				class.push("active");
			}

			let mut svg_link_paths = Vec::new();
			'link: for link in &combo.links {
				let mut path = ComboLinkPath::default();
				for point in link.points() {
					match point {
						shared::LinkPoint::Switch(switch_id, rel_x, rel_y) => match layout.switches().get(switch_id) {
							None => {
								log::error!(target: "combo", "failed to draw link for combo {}, invalid switch id {}", combo.id, switch_id);
								continue 'link;
							}
							Some(switch) => {
								let half_size = switch.size() as f64 * 0.5 + SWITCH_BORDER_WIDTH as f64;
								// get the top-left pos
								let pos = calc_switch_pos(&*window_size, switch);
								let mut pos = (pos.0 as f64, pos.1 as f64);
								// center the coords
								pos.0 += half_size;
								pos.1 += half_size;
								// apply relative offset
								pos.0 += rel_x * half_size;
								pos.1 += rel_y * half_size;
								path.push(pos);
							}
						},
						shared::LinkPoint::Point {
							pos,
							control_dirs,
							control_incoming_axis,
							control_size,
						} => {
							let control = (
								(window_size.0 as f64 * 0.5) + pos.0,
								(window_size.1 as f64 * 0.5) - pos.1,
							);
							let mut a = control;
							let mut b = control;
							if *control_incoming_axis == 0 {
								a.0 += *control_size * control_dirs.0;
								b.1 += *control_size * control_dirs.1;
							} else {
								a.1 += *control_size * control_dirs.1;
								b.0 += *control_size * control_dirs.0;
							}
							path.push_curve(a, control, b);
						}
						shared::LinkPoint::Anchor(rel_x, rel_y) => {
							let half_size = size * 0.5 + (SWITCH_BORDER_WIDTH as f64);
							let mut pos = pos;
							// center the coords
							pos.0 += half_size;
							pos.1 += half_size;
							// apply relative offset
							pos.0 += rel_x * half_size;
							pos.1 += rel_y * half_size;
							path.push(pos);
						}
					};
				}
				svg_link_paths.push(html!(<path d={path.to_string()} stroke="white" stroke-width="2" fill="none" />));
			}
			let svg_link = (!svg_link_paths.is_empty())
				.then(|| html!(<svg id={combo.id.clone()} class="link">{svg_link_paths}</svg>));

			combos.push(html!(<>
				<div id={combo.id.clone()} {class} {style}>
					<div class={classes!("slot", "center")}>
						<BindingDisplay binding={combo.label.clone()} />
					</div>
				</div>
				{svg_link}
			</>));
		}
	}

	html! {<>
		<div class="guideline x" />
		<div class="guideline y" />
		<div style="display: none;"><img src="https://raw.githubusercontent.com/tapioki/cephalopoda/main/Images/architeuthis_dux.png" style="height: 400px; margin-left: -150px; margin-top: 100px;" /></div>
		<div style={layout_style}>
			{switches}
			{combos}
		</div>
	</>}
}

fn segment_abs(segment: &svgtypes::PathSegment) -> bool {
	use svgtypes::PathSegment::*;
	match segment {
		MoveTo { abs, .. } => *abs,
		LineTo { abs, .. } => *abs,
		HorizontalLineTo { abs, .. } => *abs,
		VerticalLineTo { abs, .. } => *abs,
		CurveTo { abs, .. } => *abs,
		SmoothCurveTo { abs, .. } => *abs,
		Quadratic { abs, .. } => *abs,
		SmoothQuadratic { abs, .. } => *abs,
		EllipticalArc { abs, .. } => *abs,
		ClosePath { abs, .. } => *abs,
	}
}
fn segment_type_id(segment: &svgtypes::PathSegment) -> &'static str {
	use svgtypes::PathSegment::*;
	match segment {
		MoveTo { .. } => "m",
		LineTo { .. } => "l",
		HorizontalLineTo { .. } => "h",
		VerticalLineTo { .. } => "v",
		CurveTo { .. } => "c",
		SmoothCurveTo { .. } => "s",
		Quadratic { .. } => "q",
		SmoothQuadratic { .. } => "t",
		EllipticalArc { .. } => "a",
		ClosePath { .. } => "z",
	}
}
fn segment_id(segment: &svgtypes::PathSegment) -> String {
	let id = segment_type_id(segment);
	if segment_abs(segment) {
		id.to_uppercase()
	} else {
		id.to_owned()
	}
}
fn segment_display(segment: &svgtypes::PathSegment, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
	use svgtypes::PathSegment::*;
	write!(f, "{}", segment_id(segment))?;
	match segment {
		MoveTo { x, y, .. } => write!(f, " {x} {y}")?,
		LineTo { x, y, .. } => write!(f, " {x} {y}")?,
		HorizontalLineTo { x, .. } => write!(f, " {x}")?,
		VerticalLineTo { y, .. } => write!(f, " {y}")?,
		CurveTo {
			x1, y1, x2, y2, x, y, ..
		} => write!(f, " {x1} {y1} {x2} {y2} {x} {y}")?,
		SmoothCurveTo { x2, y2, x, y, .. } => write!(f, " {x2} {y2} {x} {y}")?,
		Quadratic { x1, y1, x, y, .. } => write!(f, " {x1} {y1} {x} {y}")?,
		SmoothQuadratic { x, y, .. } => write!(f, " {x} {y}")?,
		EllipticalArc {
			rx,
			ry,
			x_axis_rotation,
			large_arc,
			sweep,
			x,
			y,
			..
		} => {
			write!(f, " {rx} {ry} {x_axis_rotation}")?;
			write!(f, " {}", if *large_arc { 1 } else { 0 })?;
			write!(f, " {}", if *sweep { 1 } else { 0 })?;
			write!(f, " {x} {y}")?;
		}
		ClosePath { .. } => {}
	}
	Ok(())
}

#[derive(Default)]
struct ComboLinkPath(Vec<svgtypes::PathSegment>);
impl ComboLinkPath {
	fn push(&mut self, pos: (f64, f64)) {
		if self.0.is_empty() {
			self.0.push(svgtypes::PathSegment::MoveTo {
				abs: true,
				x: pos.0,
				y: pos.1,
			});
		} else {
			self.0.push(svgtypes::PathSegment::LineTo {
				abs: true,
				x: pos.0,
				y: pos.1,
			});
		}
	}

	fn push_curve(&mut self, a: (f64, f64), control: (f64, f64), b: (f64, f64)) {
		self.0.push(svgtypes::PathSegment::LineTo {
			abs: true,
			x: a.0,
			y: a.1,
		});
		self.0.push(svgtypes::PathSegment::Quadratic {
			abs: true,
			x1: control.0,
			y1: control.1,
			x: b.0,
			y: b.1,
		});
	}
}
impl std::fmt::Display for ComboLinkPath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for i in 0..self.0.len() {
			segment_display(&self.0[i], f)?;
			if i != self.0.len() - 1 {
				write!(f, " ")?;
			}
		}
		Ok(())
	}
}

#[derive(Clone, PartialEq, Properties)]
pub struct KeySwitchProps {
	pub window_size: (u32, u32),
	pub switch_id: AttrValue,
	pub switch: shared::Switch,
	pub bindings: BoundSwitch,
	pub active_slot: Option<SwitchSlot>,
}

fn calc_switch_pos(window_size: &(u32, u32), switch: &shared::Switch) -> (f64, f64) {
	let mut pos = (switch.pos.0 as f64, switch.pos.1 as f64);
	if switch.side == Some(shared::Side::Left) {
		pos.0 *= -1f64;
	}
	calculate_screen_pos(window_size, pos, switch.size() as f64)
}

fn calculate_screen_pos(window_size: &(u32, u32), mut pos: (f64, f64), size: f64) -> (f64, f64) {
	pos.0 = ((window_size.0 as f64) * 0.5) + pos.0 - (size * 0.5);
	pos.1 = ((window_size.1 as f64) * 0.5) - pos.1 - (size * 0.5);
	pos
}

static SWITCH_BORDER_WIDTH: u32 = 3;

#[function_component]
fn KeySwitch(
	KeySwitchProps {
		window_size,
		switch_id,
		switch,
		bindings,
		active_slot,
	}: &KeySwitchProps,
) -> Html {
	let mut class = classes!("switch");
	let pos = calc_switch_pos(window_size, switch);

	let style = Style::from([
		("--x", format!("{}px", pos.0)),
		("--y", format!("{}px", pos.1)),
		("width", format!("{}px", switch.size())),
		("height", format!("{}px", switch.size())),
		("border-width", format!("{SWITCH_BORDER_WIDTH}px")),
	]);

	if active_slot.is_some() {
		class.push("active");
	}

	let mut contents = Vec::new();
	for (slot, binding) in &bindings.slots {
		contents.push(html!(<SwitchSlotBinding slot={slot.clone()} binding={binding.clone()} />));
	}

	let active_slot = active_slot.as_ref().map(SwitchSlot::to_string);
	html!(<div id={switch_id.clone()} {class} {style} {active_slot}>
		{contents}
	</div>)
}

#[derive(Clone, PartialEq, Properties)]
pub struct SwitchSlotBindingProps {
	slot: SwitchSlot,
	binding: Binding,
}
#[function_component]
fn SwitchSlotBinding(SwitchSlotBindingProps { slot, binding }: &SwitchSlotBindingProps) -> Html {
	let mut class = classes!("slot");
	match slot {
		SwitchSlot::Tap => class.push("center"),
		SwitchSlot::Hold => class.push("bottom"),
	}
	let element = match &binding.display {
		None => html!(<div class="label">{binding.input.to_string()}</div>),
		Some(binding) => html!(<BindingDisplay binding={binding.clone()} />),
	};

	let layer = binding.layer.clone();
	html!(<div {class} {layer}>{element}</div>)
}

#[derive(Clone, PartialEq, Properties)]
pub struct BindingDisplayProps {
	binding: shared::BindingDisplay,
}
#[function_component]
fn BindingDisplay(BindingDisplayProps { binding }: &BindingDisplayProps) -> Html {
	match &binding {
		shared::BindingDisplay::Text(value) => html!(<div class="label">{value}</div>),
		shared::BindingDisplay::IconBootstrap(value) => html!(
			<i class={format!("bi bi-{value}")} />
		),
		shared::BindingDisplay::IconCustom(value) => html!(
			<img class={"icon"} style={format!("--glyph: url(assets/glyph/{value}.svg);")} />
		),
	}
}
