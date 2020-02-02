extern crate tiny_http;

use super::network::message::ServerApi;
use std::sync::mpsc::{self};
use std::thread;
use tiny_http::{Server, Response};
use url::Url;
use std::net::{SocketAddr};


pub struct ApiServer {
    addr: SocketAddr,
    server_api: mpsc::Sender<ServerApi>,
}

impl ApiServer {
    pub fn start(socket: SocketAddr) {
        let server = Server::http(&socket).unwrap();
        let _handler = thread::spawn(move || {
            for request in server.incoming_requests() {
                println!("received request! method: {:?}, url: {:?}, headers: {:?}",
                    request.method(),
                    request.url(),
                    request.headers()
                );

                let _ = thread::spawn(move || {
                    let url_path = request.url();
                    let mut url_base = Url::parse(&format!("http://{}/", &socket)).expect("get url base");
                    let url = url_base.join(url_path).expect("join url base and path");
                    
                    match url.path() {
                        "/server/start" => {
                            println!("receive server starts");
                        },
                        "/server/stop" => {
                            println!("receive server stop");
                        },
                        _ => {
                            println!("all other option");
                        }

                    }

                    let response = Response::from_string("hello world");
                    request.respond(response);     
                });

                
            }     
        });
    }
}


