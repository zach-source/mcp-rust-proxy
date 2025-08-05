use wasm_bindgen::JsCast;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ModalProps {
    pub show: bool,
    pub children: Children,
    pub on_background_click: Option<Callback<()>>,
}

#[function_component(Modal)]
pub fn modal(props: &ModalProps) -> Html {
    let on_background_click = {
        let callback = props.on_background_click.clone();
        Callback::from(move |e: MouseEvent| {
            let target = e.target().unwrap();
            let element: web_sys::Element = target.dyn_into().unwrap();
            if element.class_list().contains("modal") {
                if let Some(cb) = &callback {
                    cb.emit(());
                }
            }
        })
    };

    if !props.show {
        return html! {};
    }

    html! {
        <div class="modal" onclick={on_background_click}>
            {props.children.clone()}
        </div>
    }
}