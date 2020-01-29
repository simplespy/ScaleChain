use super::network::message::{Message};
use std::sync::mpsc::{self};

use futures::future;
use hyper::rt::{Future, Stream};
use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::net::{SocketAddr};

//use super::miner::miner::{ManagerMessage};
//use super::network::message::{ManagerMessage};

type BoxFut = Box<dyn Future<Item = Response<Body>, Error = hyper::Error> + Send>;


pub struct ApiServer {
    addr: String,
}

impl ApiServer {
    pub fn new(
        api_server_addr: String, 
    ) -> ApiServer {
        ApiServer {
            addr: api_server_addr,
        } 
    }

    pub fn start(mut self) {
        let addr: SocketAddr = self.addr.parse().unwrap();
        //let server = Server::bind(&addr)
        //    .serve(|| service_fn(self.command))
        //    .map_err(|e| eprintln!("server error: {}", e));
        //println!("Listening on http://{}", self.addr);
        //hyper::rt::run(server);     
    }

    fn command(&mut self, req: Request<Body>) -> BoxFut {
        let mut response = Response::new(Body::empty());

        match (req.method(), req.uri().path()) {
            // Serve some instructions at /
            (&Method::GET, "/") => {
                *response.body_mut() = Body::from("Try POSTing data to /echo");
            }
            // Convert to uppercase before sending back to client.
            (&Method::POST, "/miner/start") => {
                
            }
            (&Method::POST, "/miner/stop") => {
                
            }

            // Reverse the entire body before sending back to the client.
            //
            // Since we don't know the end yet, we can't simply stream
            // the chunks as they arrive. So, this returns a different
            // future, waiting on concatenating the full body, so that
            // it can be reversed. Only then can we return a `Response`.
            //(&Method::POST, "/miner/stop") => {
            //    let reversed = req.into_body().concat2().map(move |chunk| {
            //        let body = chunk.iter().rev().cloned().collect::<Vec<u8>>();
            //        *response.body_mut() = Body::from(body);
            //        response
            //    });
            //    return Box::new(reversed);
            //}
            // The 404 Not Found route...
            _ => {
                *response.status_mut() = StatusCode::NOT_FOUND;
            }
        };

        Box::new(future::ok(response))
    }

    
}


