mod api;
mod components;
mod types;

use components::App;

fn main() {
    yew::Renderer::<App>::new().render();
}