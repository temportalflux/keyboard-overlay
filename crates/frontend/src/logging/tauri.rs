pub struct Logger;

impl log::Log for Logger {
	fn enabled(&self, metadata: &log::Metadata) -> bool {
		metadata.level() <= ::log::Level::Trace
	}

	fn log(&self, record: &log::Record) {
		if self.enabled(record.metadata()) {
			let record = shared::LogRecord {
				level: record.level(),
				target: record.target().to_string(),
				file: record.file().map(str::to_owned),
				line: record.line(),
				args: record.args().to_string(),
			};
			crate::utility::spawn_local("logging", async move {
				tauri_sys::event::emit("log", &record).await?;
				Ok(()) as Result<(), tauri_sys::Error>
			});
		}
	}

	fn flush(&self) {}
}
