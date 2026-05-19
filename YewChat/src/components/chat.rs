use serde::{Deserialize, Serialize};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};

use crate::services::event_bus::EventBus;
use crate::{services::websocket::WebsocketService, User};

pub enum Msg {
    HandleMsg(String),
    SubmitMessage,
}

#[derive(Deserialize)]
struct MessageData {
    from: String,
    message: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MsgTypes {
    Users,
    Register,
    Message,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebSocketMessage {
    message_type: MsgTypes,
    data_array: Option<Vec<String>>,
    data: Option<String>,
}

#[derive(Clone)]
struct UserProfile {
    name: String,
    avatar: String,
}

pub struct Chat {
    users: Vec<UserProfile>,
    chat_input: NodeRef,
    _producer: Box<dyn Bridge<EventBus>>,
    wss: WebsocketService,
    messages: Vec<MessageData>,
}
impl Component for Chat {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let (user, _) = ctx
            .link()
            .context::<User>(Callback::noop())
            .expect("context to be set");
        let wss = WebsocketService::new();
        let username = user.username.borrow().clone();

        let message = WebSocketMessage {
            message_type: MsgTypes::Register,
            data: Some(username.to_string()),
            data_array: None,
        };

        if let Ok(_) = wss
            .tx
            .clone()
            .try_send(serde_json::to_string(&message).unwrap())
        {
            log::debug!("message sent successfully");
        }

        Self {
            users: vec![],
            messages: vec![],
            chat_input: NodeRef::default(),
            wss,
            _producer: EventBus::bridge(ctx.link().callback(Msg::HandleMsg)),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::HandleMsg(s) => {
                let msg: WebSocketMessage = serde_json::from_str(&s).unwrap();
                match msg.message_type {
                    MsgTypes::Users => {
                        let users_from_message = msg.data_array.unwrap_or_default();
                        self.users = users_from_message
                            .iter()
                            .map(|u| UserProfile {
                                name: u.into(),
                                avatar: format!(
                                    "https://avatars.dicebear.com/api/adventurer-neutral/{}.svg",
                                    u
                                )
                                .into(),
                            })
                            .collect();
                        return true;
                    }
                    MsgTypes::Message => {
                        let message_data: MessageData =
                            serde_json::from_str(&msg.data.unwrap()).unwrap();
                        self.messages.push(message_data);
                        return true;
                    }
                    _ => {
                        return false;
                    }
                }
            }
            Msg::SubmitMessage => {
                let input = self.chat_input.cast::<HtmlInputElement>();
                if let Some(input) = input {
                    let message = WebSocketMessage {
                        message_type: MsgTypes::Message,
                        data: Some(input.value()),
                        data_array: None,
                    };
                    if let Err(e) = self
                        .wss
                        .tx
                        .clone()
                        .try_send(serde_json::to_string(&message).unwrap())
                    {
                        log::debug!("error sending to channel: {:?}", e);
                    }
                    input.set_value("");
                };
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let submit = ctx.link().callback(|_| Msg::SubmitMessage);

        html! {
            <div class="flex w-screen h-screen" style="background: #f0f4f8;">
                <div class="flex-none w-64 h-screen flex flex-col" style="background: linear-gradient(180deg, #1e3a5f 0%, #2d6a9f 100%);">
                    <div class="p-4 border-b border-blue-400">
                        <div class="text-white text-xl font-bold">{"NaufChat"}</div>
                        <div class="text-blue-200 text-xs mt-1">{"Public Room"}</div>
                    </div>
                    <div class="text-blue-200 text-xs font-semibold px-4 pt-4 pb-2 uppercase tracking-widest">
                        {format!("Online - {}", self.users.len())}
                    </div>
                    <div class="overflow-auto flex-grow">
                    {
                        self.users.clone().iter().map(|u| {
                            html!{
                                <div class="flex items-center mx-3 mb-2 bg-white bg-opacity-10 rounded-lg p-2 hover:bg-opacity-20 transition-all">
                                    <div class="relative">
                                        <img class="w-10 h-10 rounded-full border-2 border-green-400" src={u.avatar.clone()} alt="avatar"/>
                                        <span class="absolute bottom-0 right-0 w-3 h-3 bg-green-400 rounded-full border-2 border-blue-800"></span>
                                    </div>
                                    <div class="ml-3">
                                        <div class="text-white text-sm font-medium">{u.name.clone()}</div>
                                        <div class="text-green-300 text-xs">{"Active now"}</div>
                                    </div>
                                </div>
                            }
                        }).collect::<Html>()
                    }
                    </div>
                    <div class="p-3 text-center text-blue-300 text-xs border-t border-blue-700">
                        {"Rust + WebAssembly"}
                    </div>
                </div>
                <div class="grow h-screen flex flex-col">
                    <div class="w-full h-16 flex items-center px-6 border-b border-gray-200 bg-white shadow-sm">
                        <div class="w-10 h-10 rounded-lg bg-blue-600 text-white flex items-center justify-center font-bold mr-3">{"NC"}</div>
                        <div class="min-w-0">
                            <div class="font-bold text-gray-800">{"NaufChat Room"}</div>
                            <div class="text-xs text-gray-400">{format!("{} participants", self.users.len())}</div>
                        </div>
                    </div>
                    <div class="w-full grow overflow-auto p-4" style="background: #f0f4f8;">
                        {
                            self.messages.iter().map(|m| {
                                let user = self.users.iter().find(|u| u.name == m.from).unwrap();
                                html!{
                                    <div class="flex items-end mb-4 w-4/6">
                                        <img class="w-8 h-8 rounded-full mr-3 shadow" src={user.avatar.clone()} alt="avatar"/>
                                        <div>
                                            <div class="text-xs text-gray-500 mb-1 ml-1">{m.from.clone()}</div>
                                            <div class="bg-white rounded-lg shadow-sm px-4 py-2 text-sm text-gray-700 max-w-sm">
                                                if m.message.ends_with(".gif") {
                                                    <img class="mt-1 rounded-lg" src={m.message.clone()}/>
                                                } else {
                                                    {m.message.clone()}
                                                }
                                            </div>
                                        </div>
                                    </div>
                                }
                            }).collect::<Html>()
                        }
                    </div>
                    <div class="w-full h-16 flex px-4 items-center bg-white border-t border-gray-200">
                        <input
                            ref={self.chat_input.clone()}
                            type="text"
                            placeholder="Type a message..."
                            class="block w-full py-2 px-5 bg-gray-100 rounded-full outline-none text-sm focus:ring-2 focus:ring-blue-300 transition"
                            name="message"
                            required=true
                        />
                        <button onclick={submit} class="ml-3 p-3 bg-blue-600 hover:bg-blue-700 w-10 h-10 rounded-full flex justify-center items-center shadow transition">
                            <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" class="fill-white w-5 h-5">
                                <path d="M0 0h24v24H0z" fill="none"></path><path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z"></path>
                            </svg>
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
