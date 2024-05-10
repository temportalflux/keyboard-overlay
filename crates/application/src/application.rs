// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{collections::{BTreeSet, HashMap, HashSet}, sync::{Arc, RwLock}};
use itertools::Itertools;
use shared::InputUpdate;
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
	hotkey_manager: Option<global_hotkey::GlobalHotKeyManager>,
	registered_hotkeys: multimap::MultiMap<HotKey, InputBinding>,
	active_switches: HashSet<String>,
}

#[derive(Debug, Clone)]
struct InputBinding {
	layer_id: Arc<String>,
	switch_id: Arc<String>,
	is_hold: bool,
	target_layer: Option<Arc<String>>,
	key: shared::KeyAlias,
}

impl GlobalInputState {
	fn init_app(&self, handle: tauri::AppHandle<tauri::Wry>) -> global_hotkey::Result<()> {
		let mut state = self.0.write().expect("failed to open writing on input state");
		state.app = Some(handle);
		state.hotkey_manager = Some(global_hotkey::GlobalHotKeyManager::new()?);
		drop(state);

		global_hotkey::GlobalHotKeyEvent::set_event_handler(Some({
			let handle = self.clone();
			move |event| handle.handle_event(event)
		}));

		Ok(())
	}

	fn update_bindings(&self, config: &Config) {
		self.unregister_hotkeys();
		self.insert_hotkeys(config);
		self.register_hotkeys();
	}

	fn register_hotkeys(&self) {
		let state = self.0.read().expect("failed to open writing on input state");
		let hotkeys = state.registered_hotkeys.iter_all().map(|(key, _)| key.clone()).collect::<Vec<_>>();
		if let Some(manager) = &state.hotkey_manager {
			log::debug!("{}", hotkeys.iter().map(|key| key.into_string()).join(", "));
			for hotkey in hotkeys {
				if let Err(err) = manager.register(hotkey) {
					log::error!(target: "input", "{err:?}");
				}
			}
		}
	}

	fn unregister_hotkeys(&self) {
		// default to size of a 34-count keyboard with 6 layers, a tap that shifts and a hold binding
		let mut state = self.0.write().expect("failed to open writing on input state");
		
		let hotkeys = state.registered_hotkeys.iter_all().map(|(key, _)| key.clone()).collect::<Vec<_>>();
		if let Some(manager) = &state.hotkey_manager {
			for hotkey in hotkeys {
				if let Err(err) = manager.unregister(hotkey) {
					log::error!(target: "input", "{err:?}");
				}
			}
		}

		state.registered_hotkeys.clear();
	}
	
	fn insert_hotkeys(&self, config: &Config) {
		for (layer_id, layer) in config.layout().layers() {
			log::debug!("layer: {layer_id}");
			let layer_id = Arc::new(layer_id.clone());
			for (switch_id, bindings) in layer.bindings() {
				let switch_id = Arc::new(switch_id.clone());
				if let Some(binding) = bindings.tap.as_ref() {
					let target_layer = binding.layer.as_ref().map(Clone::clone).map(Arc::new);
					self.insert_binding(InputBinding {
						layer_id: layer_id.clone(),
						switch_id: switch_id.clone(),
						is_hold: false,
						target_layer,
						key: binding.input,
					});
				}
				if let Some(binding) = bindings.hold.as_ref() {
					let target_layer = binding.layer.as_ref().map(Clone::clone).map(Arc::new);
					self.insert_binding(InputBinding {
						layer_id: layer_id.clone(),
						switch_id: switch_id.clone(),
						is_hold: true,
						target_layer,
						key: binding.input,
					});
				}
			}
		}
	}

	fn insert_binding(&self, input_binding: InputBinding) {
		let mut state = self.0.write().expect("failed to open writing on input state");
		
		for hotkey in alias_hotkeys(input_binding.key) {
			log::debug!("{:?} => {}", &input_binding.key, hotkey.into_string());
			state.registered_hotkeys.insert(hotkey, input_binding.clone());
		}
	}

	fn handle_event(&self, event: global_hotkey::GlobalHotKeyEvent) {
		log::debug!("{event:?}");
		let state = self.0.read().expect("failed to open writing on input state");
		for (hotkey, bindings) in state.registered_hotkeys.iter_all() {
			if hotkey.id() == event.id() {
				log::debug!("{} => {bindings:?}", hotkey.into_string());
				for binding in bindings {

				}
			}
		}
	}

	/*
	fn handle(&self, event: &rdev::Event) {
		let (key, is_pressed) = match event.event_type {
			rdev::EventType::KeyPress(key) => (key, true),
			rdev::EventType::KeyRelease(key) => (key, false),
			_ => return,
		};
		
		let mut state = self.0.write().expect("failed to open writing on input state");
		
		let Some(switch_id) = state.switch_bindings.get(&key) else { return };

		let was_pressed = state.active_switches.contains(&**switch_id);
		if is_pressed == was_pressed { return }

		let switch_id = switch_id.clone();
		if is_pressed {
			state.active_switches.insert(switch_id.as_ref().clone());
		}
		else
		{
			state.active_switches.remove(&*switch_id.as_ref());
		}
		
		let Some(app) = &state.app else { return };
		log::debug!("{:?}", state.active_switches);
		let _ = app.emit_all("input", InputUpdate(state.active_switches.clone()));
	}
	*/
}

fn main() -> anyhow::Result<()> {
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
		.manage(GlobalInputState::default())
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

					/*
					let _ = app.emit_all(
						"input",
						InputUpdate(["l1".into(), "r2".into(), "r4".into(), "l3".into()].into()),
					);
					*/
				}
			});

			let window = app.get_window("main").ok_or(tauri::Error::InvalidWindowHandle)?;
			//window.set_ignore_cursor_events(true)?;

			// Associate the app to global_input so that when input changes, it can be propagated to app events.
			{
				let global_input = app.state::<GlobalInputState>();
				global_input.init_app(app.handle())?;
			}
			
			// Listen for config changes to propagate them to the global input state
			app.listen_global("config", {
				let app = app.handle();
				move |event| {
					let Some(payload) = event.payload() else { return };
					let Ok(config) = serde_json::from_str(payload) else {
						return;
					};
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
											tauri::async_runtime::spawn(async move {
												log::info!("Uploading config from url {url}");
												let response = reqwest::get(url).await?;
												let contents = response.text().await?;
												upload_config(&app, &contents)?;
												// TODO: log errors
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
