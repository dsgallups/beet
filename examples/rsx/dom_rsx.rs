//! Example usage of dom rsx
//! Run in native mode to generate the static html file,
//! then build with wasm target for the interactive binary
//!
//! for live template reloading command see justfile run-dom-rsx
//!
use beet::prelude::*;
use beet::rsx::sigfault::signal;

/// The cli will run the native executable on template reloads
/// *without* recompiling.
#[cfg(not(target_arch = "wasm32"))]
fn main() {
	BeetHtml::render_to_file(app, "target/wasm-example/index.html").unwrap();
}


#[cfg(target_arch = "wasm32")]
fn main() { BeetDom::hydrate(app); }


fn app() -> RsxRoot {
	let (value, set_value) = signal(0);

	rsx! {
		<div>
			<div>"The value is "{value.clone()}</div>
			<button onclick={move |_| set_value(value() + 1)}>
				"increment"
			</button>
		</div>
	}
}
