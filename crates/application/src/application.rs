// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use shared::InputUpdate;
use std::collections::{HashMap, HashSet};
use tauri::{CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu};
use tauri_plugin_log::LogTarget;
use tauri_plugin_positioner::WindowExt;

static TRAY_OPEN_CONFIG_DIR: (&'static str, &'static str) = ("open_config_dir", "Open Config Folder");
static TRAY_LOAD_CONFIG_FILE: (&'static str, &'static str) = ("load_config", "Reload Config");

static TRAY_REFRESH_DEVICES: (&'static str, &'static str) = ("refresh_devices", "Refresh Devices");

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
impl<M, R> ManagerExt<R> for M where M: tauri::Manager<R>, R: tauri::Runtime {
	fn emit_and_trigger<S: serde::Serialize + Clone>(&self, event: &str, payload: S) -> tauri::Result<()> {
		self.trigger_global(event, Some(serde_json::to_string(&payload)?));
		self.emit_all(event, payload)
	}
}

fn main() -> anyhow::Result<()> {
	tauri::Builder::default()
		.plugin(
			tauri_plugin_log::Builder::default()
				.targets([LogTarget::LogDir, LogTarget::Stdout, LogTarget::Webview])
				.build(),
		)
		.plugin(tauri_plugin_positioner::init())
		.manage(ConfigMutex::default())
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
					let _ = app.emit_all("layout", app.state::<ConfigMutex>().get().layout);
					let _ = app.emit_all(
						"input",
						InputUpdate(["l1".into(), "r2".into(), "r4".into(), "l3".into()].into()),
					);
				}
			});

			let window = app.get_window("main").ok_or(tauri::Error::InvalidWindowHandle)?;
			window.set_ignore_cursor_events(true)?;

			// Load the config as it exists on startup
			if let Some(config) = load_config(&app.config())? {
				set_config(&app.handle(), config)?;
			}

			let tray_menu = SystemTrayMenu::new()
				.add_item(CustomMenuItem::new(MENU_TOGGLE_ID, MENU_TOGGLE_HIDE))
				.add_native_item(tauri::SystemTrayMenuItem::Separator)
				.add_item(CustomMenuItem::new(TRAY_OPEN_CONFIG_DIR.0, TRAY_OPEN_CONFIG_DIR.1))
				.add_item(CustomMenuItem::new(TRAY_LOAD_CONFIG_FILE.0, TRAY_LOAD_CONFIG_FILE.1))
				.add_native_item(tauri::SystemTrayMenuItem::Separator)
				.add_item(CustomMenuItem::new(TRAY_REFRESH_DEVICES.0, TRAY_REFRESH_DEVICES.1))
				.add_native_item(tauri::SystemTrayMenuItem::Separator)
				.add_item(CustomMenuItem::new(MENU_QUIT.0, MENU_QUIT.1));
			SystemTray::new()
				.with_menu(tray_menu)
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
								id if id == TRAY_OPEN_CONFIG_DIR.0 => {
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
								id if id == TRAY_LOAD_CONFIG_FILE.0 => {
									match load_config(&app.config()) {
										Ok(Some(config)) => if let Err(err) = set_config(&app, config) {
											log::error!("{err:?}");
										},
										Ok(None) => {},
										Err(err) => {
											log::error!("{err:?}");
										}
									}
								}
								id if id == TRAY_REFRESH_DEVICES.0 => {
									// learning hidapi: https://github.com/libusb/hidapi https://www.ontrak.net/hidapic.htm
									// could potentially read input from devices directly like this impl does
									// https://github.com/todbot/hidapitester?tab=readme-ov-file#reading-and-writing-reports
									let Ok(mut hid_api) = hidapi::HidApi::new() else { return };
									let _ = hid_api.refresh_devices();
									let mut device_map = HashMap::new();
									for info in hid_api.device_list() {
										let Some(device) = Device::from(info) else { continue };
										if !device_map.contains_key(&device) {
											device_map.insert(device.clone(), HashSet::<(u16, u16)>::default());
										}
										let Some(usage) = device_map.get_mut(&device) else {
											continue;
										};
										usage.insert((info.usage(), info.usage_page()));
									}
									log::trace!("{device_map:?}");
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

			Ok(())
		})
		.on_window_event(move |event| match event.event() {
			_ => {}
		})
		.run(tauri::generate_context!())?;
	// SetWindowsHookEx
	Ok(())
}

fn set_config(app: &tauri::AppHandle<tauri::Wry>, config: Config) -> anyhow::Result<()> {
	app.emit_all("layout", config.layout.clone())?;
	apply_initial_window_location(&app, &config)?;
	app.state::<ConfigMutex>().set(config);
	Ok(())
}

fn apply_initial_window_location(app: &tauri::AppHandle<tauri::Wry>, config: &Config) -> anyhow::Result<()> {
	let window = app.get_window("main").ok_or(tauri::Error::InvalidWindowHandle)?;

	window.set_size(tauri::PhysicalSize::<u32> {
		width: config.size.0,
		height: config.size.1,
	})?;

	move_window_to_position(&window, config.location)?;

	Ok(())
}

fn move_window_to_position(
	window: &tauri::Window,
	position: WindowPosition,
) -> anyhow::Result<()> {
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Device {
	vendor_id: u16,
	product_id: u16,
	serial: String,
	product_name: String,
	manufacturer: String,
	bus_type: BusType,
}
impl Device {
	fn from(info: &hidapi::DeviceInfo) -> Option<Self> {
		let serial = info.serial_number()?.trim();
		let product_name = info.product_string()?.trim();
		let manufacturer = info.manufacturer_string()?.trim();
		Some(Self {
			vendor_id: info.vendor_id(),
			product_id: info.product_id(),
			serial: serial.to_owned(),
			product_name: product_name.to_owned(),
			manufacturer: manufacturer.to_owned(),
			bus_type: info.bus_type().into(),
		})
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum BusType {
	Unknown = 0x00,
	Usb = 0x01,
	Bluetooth = 0x02,
	I2c = 0x03,
	Spi = 0x04,
}
impl From<hidapi::BusType> for BusType {
	fn from(value: hidapi::BusType) -> Self {
		match value {
			hidapi::BusType::Unknown => Self::Unknown,
			hidapi::BusType::Usb => Self::Usb,
			hidapi::BusType::Bluetooth => Self::Bluetooth,
			hidapi::BusType::I2c => Self::I2c,
			hidapi::BusType::Spi => Self::Spi,
		}
	}
}
