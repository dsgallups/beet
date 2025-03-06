use beet::prelude::*;
use bevy::prelude::*;

#[tokio::main]
async fn main() {
	App::new()
		.add_plugins((
			DefaultPlugins,
			BevyEventRegistry,
			BevyTemplateReloader::new(std::file!()),
			BevyRsxPlugin::new(|| {
				rsx! {
					<cam Camera2d/>
					<Counter initial=7/>
				}
			}),
		))
		.run();
}

struct Counter {
	initial: i32,
}

impl beet::prelude::Component for Counter {
	fn render(self) -> RsxRoot {
		let (get, set) = BevySignal::signal(self.initial);
		let get2 = get.clone();
		rsx! {
			<entity runtime:bevy
				Button
				onclick=move |_|set(get.clone().get() + 1)>
				"The value is "{get2}
			</entity>
		}
	}
}
