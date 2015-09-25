// #![deny(warnings)]
// #![cfg_attr(test, feature(slice_bytes))]
#![feature(collections_bound, btree_range)]

extern crate irc;
extern crate time;
#[macro_use] extern crate log;
extern crate websocket;
extern crate serde;
extern crate serde_json;

use std::io;
use std::thread;
use websocket::ws::sender::Sender;
use websocket::ws::receiver::Receiver;

pub mod serverbuf;

mod ws {
    pub use websocket::message::Message;
    pub use websocket::server::Server;
    pub use websocket::header::WebSocketProtocol as Protocol;
    use websocket::client::Client as WsClient;
    use websocket::dataframe::DataFrame as DF;
    use websocket::server::Connection;
    use websocket::server::receiver::Receiver as R;
    use websocket::server::sender::Sender as S;
    use websocket::stream::WebSocketStream as WSS;

    pub type Sender = S<WSS>;
    pub type Receiver = R<WSS>;
    pub type Conn = Connection<WSS, WSS>;
    pub type Client = WsClient<DF, S<WSS>, R<WSS>>;
}

fn start_connection(conn: io::Result<ws::Conn>) {
    let request = conn.unwrap().read_request().unwrap(); // Get the request
    let headers = request.headers.clone(); // Keep the headers so we can check them
    
    request.validate().unwrap(); // Validate the request
    
    let mut response = request.accept(); // Form a response
    
    if let Some(&ws::Protocol(ref protocols)) = headers.get() {
        if protocols.contains(&("rust-websocket".to_string())) {
            // We have a protocol we want to use
            response.headers.set(ws::Protocol(vec!["rust-websocket".to_string()]));
        }
    }
    
    let mut client = response.send().unwrap(); // Send the response
    
    let ip = client.get_mut_sender()
        .get_mut()
        .peer_addr()
        .unwrap();
    
    println!("Connection from {}", ip);
    
    let message = ws::Message::Text("Hello".to_string());
    client.send_message(message).unwrap();
    
    let (mut sender, mut receiver) = client.split();
    
    for message in receiver.incoming_messages() {
        let message = message.unwrap();
        
        match message {
            ws::Message::Close(_) => {
                let message = ws::Message::Close(None);
                sender.send_message(message).unwrap();
                println!("Client {} disconnected", ip);
                return;
            }
            ws::Message::Ping(data) => {
                let message = ws::Message::Pong(data);
                sender.send_message(message).unwrap();
            }
            _ => sender.send_message(message).unwrap(),
        }
    }
}

fn err_main() -> Result<(), i32> {
    let server = ws::Server::bind("127.0.0.1:2794").unwrap();

    for connection in server {
        // Spawn a new thread for each connection.
        thread::spawn(move || {
            start_connection(connection);
        });
    }
    Ok(())
}

fn main() {
    if let Err(err) = err_main() {
        std::process::exit(err);
    }
}