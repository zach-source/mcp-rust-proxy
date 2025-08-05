use crate::components::Modal;
use crate::types::LogData;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct LogsModalProps {
    pub show: bool,
    pub server_name: String,
    pub logs: Vec<LogData>,
    pub on_close: Callback<()>,
    pub on_clear: Callback<()>,
}

#[function_component(LogsModal)]
pub fn logs_modal(props: &LogsModalProps) -> Html {
    let auto_scroll = use_state(|| true);
    let logs_container_ref = use_node_ref();

    // Auto-scroll effect
    {
        let logs_container_ref = logs_container_ref.clone();
        let logs_len = props.logs.len();
        let auto_scroll = *auto_scroll;
        
        use_effect_with(
            (logs_len, auto_scroll),
            move |_| {
                if auto_scroll {
                    if let Some(element) = logs_container_ref.cast::<web_sys::HtmlElement>() {
                        element.set_scroll_top(element.scroll_height());
                    }
                }
            },
        );
    }

    let on_auto_scroll_change = {
        let auto_scroll = auto_scroll.clone();
        Callback::from(move |e: Event| {
            let input = e.target_unchecked_into::<HtmlInputElement>();
            auto_scroll.set(input.checked());
        })
    };

    let on_clear = {
        let callback = props.on_clear.clone();
        Callback::from(move |_| callback.emit(()))
    };

    let on_close = {
        let callback = props.on_close.clone();
        Callback::from(move |_| callback.emit(()))
    };

    html! {
        <Modal show={props.show} on_background_click={Some(props.on_close.clone())}>
            <div class="modal-content logs-modal-content">
                <div class="modal-header">
                    <h3>{"Server Logs: "}<span>{&props.server_name}</span></h3>
                    <button class="close-btn" onclick={on_close}>{"×"}</button>
                </div>
                <div class="logs-controls">
                    <button class="btn btn-secondary" onclick={on_clear}>{"Clear Logs"}</button>
                    <label class="auto-scroll-label">
                        <input
                            type="checkbox"
                            checked={*auto_scroll}
                            onchange={on_auto_scroll_change}
                        />
                        {"Auto-scroll"}
                    </label>
                </div>
                <div class="logs-container" ref={logs_container_ref}>
                    {props.logs.iter().map(|log| {
                        let level_class = log.level.as_ref()
                            .map(|l| l.to_lowercase())
                            .unwrap_or_default();
                        
                        let content = log.message.as_ref()
                            .map(|s| s.as_str())
                            .unwrap_or("");

                        html! {
                            <div class={format!("log-entry {}", level_class)}>
                                {content}
                            </div>
                        }
                    }).collect::<Html>()}
                </div>
            </div>
        </Modal>
    }
}