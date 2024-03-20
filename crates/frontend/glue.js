const invoke = window.__TAURI__ ? window.__TAURI__.invoke : undefined;

export function isBound() {
	return Boolean(window.__TAURI__);
}

export async function invokeHello(name) {
	return invoke ? await invoke("hello", {name: name}) : undefined;
}