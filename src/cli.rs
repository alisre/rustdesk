use crate::client::*;
use async_trait::async_trait;
use hbb_common::{
    config::PeerConfig,
    config::READ_TIMEOUT,
    futures::{SinkExt, StreamExt},
    log,
    message_proto::*,
    protobuf::Message as _,
    rendezvous_proto::ConnType,
    tokio::{self, sync::mpsc},
    Stream,
};
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct Session {
    id: String,
    lc: Arc<RwLock<LoginConfigHandler>>,
    sender: mpsc::UnboundedSender<Data>,
    password: String,
}

impl Session {
    pub fn new(id: &str, sender: mpsc::UnboundedSender<Data>) -> Self {
        let mut password = "".to_owned();
        if PeerConfig::load(id).password.is_empty() {
            password = rpassword::prompt_password("Enter password: ").unwrap();
        }
        let session = Self {
            id: id.to_owned(),
            sender,
            password,
            lc: Default::default(),
        };
        session.lc.write().unwrap().initialize(
            id.to_owned(),
            ConnType::PORT_FORWARD,
            None,
            false,
            None,
            None,
        );
        session
    }
}

#[async_trait]
impl Interface for Session {
    fn get_login_config_handler(&self) -> Arc<RwLock<LoginConfigHandler>> {
        return self.lc.clone();
    }

    fn msgbox(&self, msgtype: &str, title: &str, text: &str, link: &str) {
        match msgtype {
            "input-password" => {
                self.sender
                    .send(Data::Login((self.password.clone(), true)))
                    .ok();
            }
            "re-input-password" => {
                log::error!("{}: {}", title, text);
                match rpassword::prompt_password("Enter password: ") {
                    Ok(password) => {
                        let login_data = Data::Login((password, true));
                        self.sender.send(login_data).ok();
                    }
                    Err(e) => {
                        log::error!("reinput password failed, {:?}", e);
                    }
                }
            }
            msg if msg.contains("error") => {
                log::error!("{}: {}: {}", msgtype, title, text);
            }
            _ => {
                log::info!("{}: {}: {}", msgtype, title, text);
            }
        }
    }

    fn handle_login_error(&self, err: &str) -> bool {
        handle_login_error(self.lc.clone(), err, self)
    }

    fn handle_peer_info(&self, pi: PeerInfo) {
        self.lc.write().unwrap().handle_peer_info(&pi);
    }

    async fn handle_hash(&self, pass: &str, hash: Hash, peer: &mut Stream) {
        log::info!(
            "password={}",
            hbb_common::password_security::temporary_password()
        );
        handle_hash(self.lc.clone(), &pass, hash, self, peer).await;
    }

    async fn handle_login_from_ui(
        &self,
        os_username: String,
        os_password: String,
        password: String,
        remember: bool,
        peer: &mut Stream,
    ) {
        handle_login_from_ui(
            self.lc.clone(),
            os_username,
            os_password,
            password,
            remember,
            peer,
        )
        .await;
    }

    async fn handle_test_delay(&self, t: TestDelay, peer: &mut Stream) {
        handle_test_delay(t, peer).await;
    }

    fn send(&self, data: Data) {
        self.sender.send(data).ok();
    }
}

#[tokio::main(flavor = "current_thread")]
pub async fn connect_test(id: &str, key: String, token: String) {
    let (sender, mut receiver) = mpsc::unbounded_channel::<Data>();
    let handler = Session::new(&id, sender);
    match crate::client::Client::start(id, &key, &token, ConnType::PORT_FORWARD, handler).await {
        Err(err) => {
            log::error!("Failed to connect {}: {}", &id, err);
        }
        Ok((mut stream, direct)) => {
            log::info!("direct: {}", direct);
            // rpassword::prompt_password("Input anything to exit").ok();
            loop {
                tokio::select! {
                    res = hbb_common::timeout(READ_TIMEOUT, stream.next()) => match res {
                        Err(_) => {
                            log::error!("Timeout");
                            break;
                        }
                        Ok(Some(Ok(bytes))) => {
                            if let Ok(msg_in) = Message::parse_from_bytes(&bytes) {
                                match msg_in.union {
                                    Some(message::Union::Hash(hash)) => {
                                        log::info!("Got hash");
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

#[tokio::main(flavor = "current_thread")]
pub async fn start_one_port_forward(
    id: String,
    port: i32,
    remote_host: String,
    remote_port: i32,
    key: String,
    token: String,
) {
    crate::common::test_rendezvous_server();
    crate::common::test_nat_type();
    let (sender, mut receiver) = mpsc::unbounded_channel::<Data>();
    let handler = Session::new(&id, sender);
    if let Err(err) = crate::port_forward::listen(
        handler.id.clone(),
        handler.password.clone(),
        port,
        handler.clone(),
        receiver,
        &key,
        &token,
        handler.lc.clone(),
        remote_host,
        remote_port,
    )
    .await
    {
        log::error!("Failed to listen on {}: {}", port, err);
    }
    log::info!("port forward (:{}) exit", port);
}

pub fn login(username: String, password: String) {
    use hbb_common::config::LocalConfig;
    use serde_json::json;
    
    log::info!("Attempting to login with username: {}", username);
    
    // Get API server
    let api_server = hbb_common::config::Config::get_option("api-server");
    if api_server.is_empty() {
        log::error!("API server not configured");
        println!("Error: API server not configured. Please set it in settings.");
        return;
    }
    
    log::info!("Using API server: {}", api_server);
    
    // Prepare login request
    let login_url = format!("{}/api/login", api_server);
    let device_info = json!({
        "id": hbb_common::config::Config::get_id(),
        // Avoid UI/GTK dependencies in CLI mode.
        "uuid": crate::encode64(hbb_common::get_uuid()),
    });
    
    let login_data = json!({
        "username": username,
        "password": password,
        "id": device_info["id"],
        "uuid": device_info["uuid"],
        "type": "account",
    });
    
    // Make HTTP request (blocking)
    let client = reqwest::blocking::Client::new();
    match client
        .post(&login_url)
        .header("Content-Type", "application/json")
        .json(&login_data)
        .send()
    {
        Ok(response) => {
            let status = response.status();
            match response.text() {
                Ok(body) => {
                    if status.is_success() {
                        match serde_json::from_str::<serde_json::Value>(&body) {
                            Ok(json_body) => {
                                if let Some(error) = json_body.get("error") {
                                    log::error!("Login failed: {}", error);
                                    println!("Login failed: {}", error);
                                } else if let Some(access_token) = json_body.get("access_token") {
                                    if let Some(token_str) = access_token.as_str() {
                                        // Save access token
                                        LocalConfig::set_option("access_token".to_string(), token_str.to_string());
                                        
                                        // Save user info
                                        if let Some(user) = json_body.get("user") {
                                            LocalConfig::set_option("user_info".to_string(), user.to_string());
                                            if let Some(name) = user.get("name") {
                                                log::info!("Login successful! Welcome {}", name);
                                                println!("Login successful! Welcome {}", name);
                                            } else {
                                                log::info!("Login successful!");
                                                println!("Login successful!");
                                            }
                                        } else {
                                            log::info!("Login successful!");
                                            println!("Login successful!");
                                        }
                                    } else {
                                        log::error!("Invalid access token format");
                                        println!("Error: Invalid access token format");
                                    }
                                } else {
                                    log::error!("No access token in response");
                                    println!("Error: No access token in response");
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to parse response: {}", e);
                                println!("Error: Failed to parse response: {}", e);
                            }
                        }
                    } else {
                        log::error!("Login failed with status {}: {}", status, body);
                        println!("Login failed with status {}: {}", status, body);
                    }
                }
                Err(e) => {
                    log::error!("Failed to read response body: {}", e);
                    println!("Error: Failed to read response body: {}", e);
                }
            }
        }
        Err(e) => {
            log::error!("Failed to connect to API server: {}", e);
            println!("Error: Failed to connect to API server: {}", e);
        }
    }
}
