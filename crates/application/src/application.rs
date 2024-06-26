// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use multimap::MultiMap;
use std::{
	collections::{BTreeSet, HashMap, HashSet},
	sync::{Arc, RwLock},
};
use tauri::{CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTraySubmenu};
use tauri_plugin_log::LogTarget;
use tauri_plugin_positioner::WindowExt;

static TRAY_CONFIG_IMPORT: (&'static str, &'static str) = ("config:import", "Import Config");
static TRAY_CONFIG_EXPORT: (&'static str, &'static str) = ("config:export", "Export Config");
static TRAY_CONFIG_OPEN_DIR: (&'static str, &'static str) = ("open_config_dir", "Open Config Folder");
static TRAY_CONFIG_RELOAD: (&'static str, &'static str) = ("load_config", "Reload Config");

static MENU_TOGGLE_ID: &'static str = "toggle";
static MENU_TOGGLE_HIDE: &'static str = "Hide";
static MENU_TOGGLE_SHOW: &'static str = "Show";
static EVENT_TOGGLE_WINDOW_VISIBILITY: &'static str = "toggle_window_visibility";

static MENU_QUIT: (&'static str, &'static str) = ("quit", "Quit");

mod config;
pub use config::*;

trait ManagerExt<R: tauri::Runtime> {
	fn emit_and_trigger<S: serde::Serialize + Clone>(&self, event: &str, payload: S) -> tauri::Result<()>;
}
impl<M, R> ManagerExt<R> for M
where
	M: tauri::Manager<R>,
	R: tauri::Runtime,
{
	fn emit_and_trigger<S: serde::Serialize + Clone>(&self, event: &str, payload: S) -> tauri::Result<()> {
		self.trigger_global(event, Some(serde_json::to_string(&payload)?));
		self.emit_all(event, payload)
	}
}

#[derive(Clone, Default)]
struct GlobalInputState(Arc<RwLock<InputState>>);
#[derive(Default)]
struct InputState {
	app: Option<tauri::AppHandle<tauri::Wry>>,
	layer_order: Vec<String>,
	layer_switches: HashMap<String, HashSet<String>>,

	key_to_relevant_hotkeys: MultiMap<rdev::Key, HotKey>,
	hotkey_bindings: MultiMap<HotKey, InputBinding>,

	pressed_keys: HashSet<rdev::Key>,
	pressed_hotkeys: HashSet<HotKey>,

	default_layer: String,
	active_layers: HashSet<String>,
	active_switches: BTreeSet<String>,
}

#[derive(Debug, Clone)]
struct InputBinding {
	layer_id: HashSet<Arc<String>>,
	switch_id: Arc<String>,
	slot: Option<shared::SwitchSlot>,
	key: shared::KeySet,
	target_layer: Option<Arc<String>>,
}

impl InputState {
	fn can_trigger(&self, binding: &InputBinding) -> bool {
		for layer_id in self.layer_order.iter().rev() {
			// The layer being scanned is not active
			if !self.active_layers.contains(layer_id) {
				continue;
			}
			// We found our layer, so it must be able to trigger
			if binding.layer_id.contains(layer_id) {
				return true;
			}
			// This is some layer with higher priority than the binding, so see if this layer blocks it
			let Some(bound_switches) = self.layer_switches.get(layer_id) else {
				continue;
			};
			if bound_switches.contains(&*binding.switch_id) {
				// something else has the switch bound
				return false;
			}
		}
		false
	}
}

impl GlobalInputState {
	fn init_app(&self, handle: tauri::AppHandle<tauri::Wry>) {
		let mut state = self.0.write().expect("failed to open writing on input state");
		state.app = Some(handle);
	}

	fn update_bindings(&self, config: &Config) {
		{
			let mut state = self.0.write().expect("failed to open writing on input state");

			let default_layer = config.layout().default_layer();
			state.default_layer = default_layer.clone();
			state.active_layers.insert(default_layer.clone());

			state.layer_order = config.layout().layer_order().clone();
			state.layer_switches.clear();
			for (layer_id, layer) in config.layout().layers() {
				let switch_ids = layer.bindings().keys().map(Clone::clone).collect();
				state.layer_switches.insert(layer_id.clone(), switch_ids);
			}

			state.key_to_relevant_hotkeys.clear();
			state.hotkey_bindings.clear();
			state.pressed_keys.clear();
			state.pressed_hotkeys.clear();
		}
		self.insert_hotkeys(config);
	}

	fn insert_hotkeys(&self, config: &Config) {
		for (layer_id, layer) in config.layout().layers() {
			let layer_id = Arc::new(layer_id.clone());
			for (switch_id, bindings) in layer.bindings() {
				let switch_id = Arc::new(switch_id.clone());
				for (slot, binding) in &bindings.slots {
					let target_layer = binding.layer.as_ref().map(Clone::clone).map(Arc::new);
					self.insert_binding(InputBinding {
						layer_id: [layer_id.clone()].into(),
						switch_id: switch_id.clone(),
						slot: Some(*slot),
						target_layer,
						key: binding.input.clone(),
					});
				}
			}
		}
		for combo in config.layout().combos() {
			let target_layer = combo.input_layer.as_ref().map(Clone::clone).map(Arc::new);
			self.insert_binding(InputBinding {
				layer_id: HashSet::default(),
				switch_id: Arc::new(combo.id.clone()),
				slot: None,
				target_layer,
				key: combo.input.clone(),
			});
		}
	}

	fn insert_binding(&self, input_binding: InputBinding) {
		let mut state = self.0.write().expect("failed to open writing on input state");
		for hotkey in alias_hotkeys(&input_binding.key) {
			for code in hotkey.relevant_keys() {
				state.key_to_relevant_hotkeys.insert(code, hotkey);
			}
			state.hotkey_bindings.insert(hotkey, input_binding.clone());
		}
	}

	fn handle(&self, event: &rdev::Event) {
		let mut state = self.0.write().expect("failed to open writing on input state");
		let key = match event.event_type {
			rdev::EventType::KeyPress(key) => {
				state.pressed_keys.insert(key);
				key
			}
			rdev::EventType::KeyRelease(key) => {
				state.pressed_keys.remove(&key);
				key
			}
			_ => return,
		};

		let Some(hotkeys) = state.key_to_relevant_hotkeys.get_vec(&key).cloned() else {
			return;
		};

		let mut changed_hotkeys = HashSet::with_capacity(10);
		for hotkey in hotkeys {
			if hotkey.is_pressed(&state.pressed_keys) {
				if state.pressed_hotkeys.insert(hotkey) {
					changed_hotkeys.insert(hotkey);
				}
			} else {
				if state.pressed_hotkeys.remove(&hotkey) {
					changed_hotkeys.insert(hotkey);
				}
			}
		}

		let mut updates = Vec::new();
		for hotkey in changed_hotkeys {
			let pressed = state.pressed_hotkeys.contains(&hotkey);
			if let Some(bindings) = state.hotkey_bindings.get_vec(&hotkey).cloned() {
				for binding in bindings {
					if pressed && state.can_trigger(&binding) {
						if let Some(new_layer) = &binding.target_layer {
							updates.push(shared::InputUpdate::LayerActivate((**new_layer).clone()));
						}
						updates.push(shared::InputUpdate::SwitchPressed(
							(*binding.switch_id).clone(),
							binding.slot,
						));
					} else if !pressed {
						if let Some(layer) = &binding.target_layer {
							updates.push(shared::InputUpdate::LayerDeactivate((**layer).clone()));
						}
						updates.push(shared::InputUpdate::SwitchReleased((*binding.switch_id).clone()));
					}
				}
			}
		}

		for update in updates {
			match &update {
				shared::InputUpdate::LayerActivate(layer) => {
					state.active_layers.insert(layer.clone());
				}
				shared::InputUpdate::LayerDeactivate(layer) => {
					state.active_layers.remove(layer);
				}
				shared::InputUpdate::SwitchPressed(switch_id, _slot) => {
					state.active_switches.insert(switch_id.clone());
				}
				shared::InputUpdate::SwitchReleased(switch_id) => {
					state.active_switches.remove(switch_id);
				}
			}

			if let Some(app) = &state.app {
				let _ = app.emit_all("input", update);
			}
		}
	}
}

fn main() -> anyhow::Result<()> {
	let global_input = GlobalInputState::default();
	std::thread::spawn({
		let input = global_input.clone();
		move || {
			if let Err(err) = rdev::grab(move |event| {
				input.handle(&event);
				Some(event)
			}) {
				log::error!(target: "rdev", "{err:?}");
			}
		}
	});

	tauri::Builder::default()
		.plugin(
			tauri_plugin_log::Builder::default()
				.targets([LogTarget::LogDir, LogTarget::Stdout, LogTarget::Webview])
				.filter(|record| {
					static IGNORED_TARGETS: [&'static str; 1] = ["hyper_util"];
					for ignored in IGNORED_TARGETS {
						if record.target().contains(ignored) {
							return false;
						}
					}
					true
				})
				.build(),
		)
		.plugin(tauri_plugin_positioner::init())
		.plugin(tauri_plugin_clipboard::init())
		.manage(ConfigMutex::default())
		.manage(global_input)
		.setup(|app| {
			// Listen for logging from the frontend
			app.listen_global("log", |event| {
				let Some(payload_str) = event.payload() else { return };
				let Ok(record) = serde_json::from_str::<shared::LogRecord>(payload_str) else {
					return;
				};
				log::log!(target: record.target.as_str(), record.level, "{}", record.args);
			});
			// Wait for the frontend to become ready
			app.listen_global("ready", {
				let app = app.handle();
				move |_| {
					log::info!("received ready event from frontened");
					let config = app.state::<ConfigMutex>().get();

					let icon_scale = config.active_profile().map(|profile| profile.scale).unwrap_or(1.0);
					let _ = app.emit_all("scale", icon_scale);

					let _ = app.emit_all("layout", config.layout().clone());
					let _ = app.emit_all(
						"input",
						shared::InputUpdate::LayerActivate(config.layout().default_layer().clone()),
					);
				}
			});

			let window = app.get_window("main").ok_or(tauri::Error::InvalidWindowHandle)?;

			// If not in debug mode, then ignore cursor events on the window
			if !cfg!(debug_assertions) {
				window.set_ignore_cursor_events(true)?;
			}

			// Associate the app to global_input so that when input changes, it can be propagated to app events.
			{
				let global_input = app.state::<GlobalInputState>();
				global_input.init_app(app.handle());
			}

			// Listen for config changes to propagate them to the global input state
			app.listen_global("config", {
				let app = app.handle();
				move |event| {
					let Some(payload) = event.payload() else { return };
					let Ok(config) = serde_json::from_str::<Config>(payload) else {
						return;
					};
					let _ = app.emit_all("layout", config.layout().clone());
					let global_input = app.state::<GlobalInputState>();
					global_input.update_bindings(&config);
				}
			});

			// Load the config as it exists on startup
			if let Some(config) = load_config(&app.config())? {
				if let Some(profile) = config.active_profile() {
					apply_initial_window_location(&app.handle(), profile)?;
				}
				set_config(&app.handle(), config)?;
			}

			SystemTray::new()
				.with_menu(build_system_tray_menu(&app.state::<ConfigMutex>().get()))
				.on_event({
					let app = app.handle();
					move |event| {
						tauri_plugin_positioner::on_tray_event(&app, &event);
						match event {
							SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
								id if id == MENU_QUIT.0 => {
									app.exit(0);
								}
								id if id == MENU_TOGGLE_ID => {
									let Some(window) = app.get_window("main") else { return };
									window.trigger(EVENT_TOGGLE_WINDOW_VISIBILITY, None);
								}
								id if id == TRAY_CONFIG_OPEN_DIR.0 => {
									let Some(config_dir) = tauri::api::path::app_config_dir(&app.config()) else {
										return;
									};
									let config_path_str = config_dir.display().to_string();
									let Err(err) = tauri::api::shell::open(&app.shell_scope(), &config_path_str, None)
									else {
										return;
									};
									log::error!("failed to open config directory {config_path_str:?}: {err:?}");
								}
								id if id == TRAY_CONFIG_RELOAD.0 => match load_config(&app.config()) {
									Ok(Some(config)) => {
										if let Err(err) = set_config(&app, config) {
											log::error!("{err:?}");
										}
									}
									Ok(None) => {}
									Err(err) => {
										log::error!("{err:?}");
									}
								},
								id if id.starts_with("profile:") => {
									let Some(profile_name) = id.strip_prefix("profile:") else {
										return;
									};
									let config_state = app.state::<ConfigMutex>();
									let mut config = config_state.get();
									let Ok(()) = config.set_active_profile(profile_name) else {
										return;
									};
									let Ok(config_payload) = serde_json::to_string(&config) else {
										return;
									};
									let _ = save_config(&app.config(), &config);
									config_state.set(config);
									app.trigger_global("config:profile", Some(config_payload));
								}
								id if id == TRAY_CONFIG_IMPORT.0 => {
									let clipboard = app.state::<tauri_plugin_clipboard::ClipboardManager>();

									if let Ok(clipboard_file_path_strs) = clipboard.read_files() {
										if let Some(file_path_str) = clipboard_file_path_strs.first() {
											log::info!("Uploading config from local file {file_path_str:?}");
											if let Ok(contents) = tauri::api::file::read_string(&file_path_str) {
												let _ = upload_config(&app, &contents);
											}
										}
									} else if let Ok(clipboard_text) = clipboard.read_text() {
										if let Ok(url) = reqwest::Url::parse(&clipboard_text) {
											let app = app.clone();
											spawn("config", async move {
												log::info!("Uploading config from url {url}");
												let response = reqwest::get(url).await?;
												let contents = response.text().await?;
												upload_config(&app, &contents)?;
												Ok(()) as anyhow::Result<()>
											});
										} else {
											log::info!("Uploading config contents from clipboard");
											let _ = upload_config(&app, &clipboard_text);
										}
									}
								}
								id if id == TRAY_CONFIG_EXPORT.0 => {
									let config_state = app.state::<ConfigMutex>();
									let mut config = config_state.get();
									// prep for export, clearing runtime data
									config.clear_state();

									let clipboard = app.state::<tauri_plugin_clipboard::ClipboardManager>();
									let _ = clipboard.write_text(serialize_config_kdl(&config));
								}
								_ => {}
							},
							_ => {}
						}
					}
				})
				.build(app)?;

			// Handle toggling the window visibility
			window.listen(EVENT_TOGGLE_WINDOW_VISIBILITY, {
				let app = app.handle();
				move |_event| {
					let Some(window) = app.get_window("main") else { return };
					let Ok(is_visible) = window.is_visible() else { return };
					let menu_item = app.tray_handle().get_item(MENU_TOGGLE_ID);
					if is_visible {
						let Ok(_) = window.hide() else { return };
						let _ = menu_item.set_title(MENU_TOGGLE_SHOW);
					} else {
						let Ok(_) = window.show() else { return };
						let _ = menu_item.set_title(MENU_TOGGLE_HIDE);
					}
				}
			});

			// When the config loads, rebuild the system tray menu (to account for display profiles loading)
			app.listen_global("config", {
				let app_handle = app.handle();
				move |event| {
					let Some(payload) = event.payload() else { return };
					let Ok(config) = serde_json::from_str(payload) else {
						return;
					};
					let _ = app_handle.tray_handle().set_menu(build_system_tray_menu(&config));
				}
			});

			// When the config loads or the active display profile is changed, adjust the window accordingly
			app.listen_global("config:profile", {
				let app = app.handle();
				move |event| {
					let Some(payload) = event.payload() else { return };
					let Ok(config) = serde_json::from_str::<Config>(payload) else {
						return;
					};
					let Some(profile) = config.active_profile() else { return };
					let _ = apply_initial_window_location(&app, profile);
					let _ = app.emit_all("scale", profile.scale);
				}
			});

			Ok(())
		})
		.run(tauri::generate_context!())?;
	Ok(())
}

fn upload_config(app: &tauri::AppHandle<tauri::Wry>, contents: &str) -> anyhow::Result<()> {
	let config = parse_config_kdl(contents)?;
	save_config(&app.config(), &config)?;
	set_config(&app, config)?;
	Ok(())
}

fn build_system_tray_menu(config: &Config) -> SystemTrayMenu {
	let mut menu = SystemTrayMenu::new();
	menu = menu.add_item(CustomMenuItem::new(MENU_TOGGLE_ID, MENU_TOGGLE_HIDE));

	if config.has_profiles() {
		menu = menu.add_submenu(SystemTraySubmenu::new(
			"Profiles",
			config
				.iter_profiles()
				.fold(SystemTrayMenu::new(), |menu, (name, _profile)| {
					menu.add_item(CustomMenuItem::new(format!("profile:{name}"), name))
				}),
		));
	}

	menu.add_native_item(tauri::SystemTrayMenuItem::Separator)
		.add_item(CustomMenuItem::new(TRAY_CONFIG_IMPORT.0, TRAY_CONFIG_IMPORT.1))
		.add_item(CustomMenuItem::new(TRAY_CONFIG_EXPORT.0, TRAY_CONFIG_EXPORT.1))
		.add_item(CustomMenuItem::new(TRAY_CONFIG_RELOAD.0, TRAY_CONFIG_RELOAD.1))
		.add_item(CustomMenuItem::new(TRAY_CONFIG_OPEN_DIR.0, TRAY_CONFIG_OPEN_DIR.1))
		.add_native_item(tauri::SystemTrayMenuItem::Separator)
		.add_item(CustomMenuItem::new(MENU_QUIT.0, MENU_QUIT.1))
}

fn set_config(app: &tauri::AppHandle<tauri::Wry>, config: Config) -> anyhow::Result<()> {
	app.emit_all("layout", config.layout().clone())?;

	let config_payload = serde_json::to_string(&config)?;
	app.state::<ConfigMutex>().set(config);
	app.trigger_global("config", Some(config_payload.clone()));
	app.trigger_global("config:profile", Some(config_payload));
	Ok(())
}

fn apply_initial_window_location(app: &tauri::AppHandle<tauri::Wry>, profile: &DisplayProfile) -> anyhow::Result<()> {
	let window = app.get_window("main").ok_or(tauri::Error::InvalidWindowHandle)?;

	window.set_size(tauri::PhysicalSize::<u32> {
		width: (profile.size.0 as f64 * profile.scale).floor() as u32,
		height: (profile.size.1 as f64 * profile.scale).floor() as u32,
	})?;

	move_window_to_position(&window, profile.location)?;

	Ok(())
}

fn move_window_to_position(window: &tauri::Window, position: WindowPosition) -> anyhow::Result<()> {
	// Move the window to the correct monitor
	let monitors = window.available_monitors()?;
	let monitor = usize::min(position.monitor, monitors.len());
	if let Some(monitor) = monitors.get(monitor) {
		window.set_position(monitor.position().clone())?;
	}
	// Anchor it relative to that monitor
	window.move_window(position.anchor.into())?;
	// And offset it from the anchor by some amount
	window.set_position({
		let mut pos = window.outer_position()?;
		pos.x += position.offset.0;
		pos.y -= position.offset.1;
		pos
	})?;
	Ok(())
}

pub fn spawn<F, E>(target: &'static str, future: F)
where
	F: futures::Future<Output = Result<(), E>> + 'static + Send,
	E: 'static + std::fmt::Debug,
{
	tauri::async_runtime::spawn(async move {
		let Err(err) = future.await else { return };
		log::error!(target: target, "{err:?}");
	});
}
