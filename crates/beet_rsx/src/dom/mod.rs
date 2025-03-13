use crate::prelude::*;
#[cfg(target_arch = "wasm32")]
mod beet_dom;
#[cfg(target_arch = "wasm32")]
mod dom_event_registry;
#[cfg(target_arch = "wasm32")]
pub use beet_dom::*;
#[cfg(target_arch = "wasm32")]
pub use dom_event_registry::EventRegistry;
#[cfg(not(target_arch = "wasm32"))]
mod native_event_registry;
mod rs_dom_target;
#[cfg(not(target_arch = "wasm32"))]
pub use native_event_registry::EventRegistry;
pub use rs_dom_target::*;
use std::cell::RefCell;


#[cfg(target_arch = "wasm32")]
mod browser_dom_target;
#[cfg(target_arch = "wasm32")]
pub use browser_dom_target::*;

thread_local! {
	#[rustfmt::skip]
	static DOM_TARGET: RefCell<Box<dyn DomTargetImpl>> =
		RefCell::new(Box::new(RsDomTarget::new(&().into_root()).unwrap()));
}

/// Mechanism for swapping out:
/// - [`BrowserDomTarget`]
/// - [`RsDomTarget`]
pub struct DomTarget;

impl DomTarget {
	pub fn with<R>(mut func: impl FnMut(&mut dyn DomTargetImpl) -> R) -> R {
		DOM_TARGET.with(|current| {
			let mut current = current.borrow_mut();
			func(current.as_mut())
		})
	}


	pub fn set(item: impl 'static + Sized + DomTargetImpl) {
		DOM_TARGET.with(|current| {
			*current.borrow_mut() = Box::new(item);
		});
	}
}

pub trait DomTargetImpl {
	fn html_constants(&self) -> &HtmlConstants;

	// type Event;
	fn update_rsx_node(
		&mut self,
		node: RsxNode,
		loc: TreeLocation,
	) -> ParseResult<()>;

	// TODO update attriute block, update block value


	/// just used for testing atm
	fn render(&self) -> String;

	// fn register_event(&self, event: Box<dyn Fn() -> ()>);
}
