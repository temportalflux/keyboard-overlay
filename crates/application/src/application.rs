// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, WindowEvent};
use tauri_plugin_positioner::WindowExt;

static MENU_QUIT: &'static str = "quit";
static MENU_TOGGLE: &'static str = "toggle";
static EVENT_TOGGLE_WINDOW_VISIBILITY: &'static str = "toggleWindowVisibility";

#[tauri::command]
fn hello(name: &str) -> Result<String, String> {
	// This is a very simplistic example but it shows how to return a Result
	// and use it in the front-end.
	if name.contains(' ') {
		Err("Name should not contain spaces".to_string())
	} else {
		Ok(format!("Hello, {}", name))
	}
}

fn main() -> Result<(), tauri::Error> {
	tauri::Builder::default()
		.plugin(tauri_plugin_positioner::init())
		.invoke_handler(tauri::generate_handler![hello])
		.setup(|app| {
			let window = app.get_window("main").ok_or(tauri::Error::InvalidWindowHandle)?;

			let tray_menu = SystemTrayMenu::new()
				.add_item(CustomMenuItem::new(MENU_QUIT, "Quit"))
				.add_item(CustomMenuItem::new(MENU_TOGGLE, "Hide"));
			SystemTray::new()
				.with_menu(tray_menu)
				.on_event({
					let app = app.handle();
					move |event| {
						tauri_plugin_positioner::on_tray_event(&app, &event);
						match event {
							SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
								id if id == MENU_QUIT => {
									app.exit(0);
								}
								id if id == MENU_TOGGLE => {
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

			window.listen(EVENT_TOGGLE_WINDOW_VISIBILITY, {
				let app = app.handle();
				move |event| {
					let Some(window) = app.get_window("main") else { return };
					let Ok(is_visible) = window.is_visible() else { return };
					let menu_item = app.tray_handle().get_item(MENU_TOGGLE);
					if is_visible {
						let Ok(_) = window.hide() else { return };
						let _ = menu_item.set_title("Show");
					}
					else {
						let Ok(_) = window.show() else { return };
						let _ = menu_item.set_title("Hide");
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
