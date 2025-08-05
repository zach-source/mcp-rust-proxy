use crate::components::ServerCard;
use crate::types::Server;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ServersListProps {
    pub servers: Vec<Server>,
    pub on_action: Callback<(String, String)>,
    pub on_view_logs: Callback<String>,
}

#[function_component(ServersList)]
pub fn servers_list(props: &ServersListProps) -> Html {
    html! {
        <section class="servers-section">
            <h2>{"Servers"}</h2>
            <div class="servers-list">
                {props.servers.iter().map(|server| {
                    html! {
                        <ServerCard
                            key={server.name.clone()}
                            server={server.clone()}
                            on_action={props.on_action.clone()}
                            on_view_logs={props.on_view_logs.clone()}
                        />
                    }
                }).collect::<Html>()}
            </div>
        </section>
    }
}