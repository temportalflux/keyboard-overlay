// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use shared::{InputUpdate, Layout};
use tauri::{CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu};
use tauri_plugin_log::LogTarget;
use tauri_plugin_positioner::WindowExt;

static MENU_QUIT: (&'static str, &'static str) = ("quit", "Quit");
static MENU_TOGGLE_ID: &'static str = "toggle";
static MENU_TOGGLE_HIDE: &'static str = "Hide";
static MENU_TOGGLE_SHOW: &'static str = "Show";
static EVENT_TOGGLE_WINDOW_VISIBILITY: &'static str = "toggleWindowVisibility";

fn main() -> Result<(), tauri::Error> {
	tauri::Builder::default()
		.plugin(
			tauri_plugin_log::Builder::default()
				.targets([LogTarget::LogDir, LogTarget::Stdout, LogTarget::Webview])
				.build(),
		)
		.plugin(tauri_plugin_positioner::init())
		.setup(|app| {
			// Listen for logging from the frontend
			app.listen_global("log", |event| {
				let Some(payload_str) = event.payload() else { return };
				let Ok(record) = serde_json::from_str::<shared::LogRecord>(payload_str) else { return };
				log::log!(target: record.target.as_str(), record.level, "{}", record.args);
			});
			// Wait for the frontend to become ready
			app.once_global("ready", {
				let app = app.handle();
				move |_| {
					log::debug!("received ready event from frontened");
					log::debug!("emitting initialization events");
					let _ = app.emit_all("layout", Layout);
					let _ = app.emit_all("input", InputUpdate);
				}
			});

			let window = app.get_window("main").ok_or(tauri::Error::InvalidWindowHandle)?;

			let tray_menu = SystemTrayMenu::new()
				.add_item(CustomMenuItem::new(MENU_TOGGLE_ID, MENU_TOGGLE_HIDE))
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
								_ => {}
							},
							_ => {}
						}
					}
				})
				.build(app)?;

			window.move_window(tauri_plugin_positioner::Position::BottomLeft)?;
			window.set_ignore_cursor_events(true)?;

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
	Ok(())
}
