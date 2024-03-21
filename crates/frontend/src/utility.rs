use futures::Future;

pub fn spawn_local<F, E>(target: &'static str, future: F)
where
	F: Future<Output = Result<(), E>> + 'static,
	E: 'static + std::fmt::Debug,
{
	wasm_bindgen_futures::spawn_local(async move {
		let Err(err) = future.await else { return };
		log::error!(target: target, "{err:?}");
	});
}
