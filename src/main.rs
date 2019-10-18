extern crate futures;
extern crate tokio;
extern crate websocket;

pub mod types;

use tokio::codec::Framed;
use tokio::net::TcpStream;
use tokio::runtime;
use tokio::runtime::TaskExecutor;

use std::collections::HashMap;

use websocket::message::OwnedMessage;
use websocket::r#async::MessageCodec;
use websocket::server::r#async::Server;
use websocket::server::InvalidConnection;

use futures::stream::SplitSink;
use futures::{Future, Sink, Stream};

use std::sync::{Arc, RwLock};

use std::collections::hash_map::Entry::{Occupied, Vacant};

fn main() {
    let runtime = runtime::Builder::new().build().unwrap();
    let executor = runtime.executor();
    let server =
        Server::bind("127.0.0.1:8080", &runtime.reactor()).expect("Failed to create server");

    // Hashmap to store a sink value with an id key
    // A sink is used to send data to an open client connection
    let connections = Arc::new(RwLock::new(HashMap::new()));
    // Hashmap of id:entity pairs. This is basically the game state
    let entities = Arc::new(RwLock::new(Vec::new()));
    // Used to assign a unique id to each new player
    let counter = Arc::new(RwLock::new(0));

    // Clone references to these states in order to move into the connection_handler closure
    let connections_inner = connections.clone();
    let entities_inner = entities.clone();
    let executor_inner = executor.clone();

    // This stream spawns a future on each new connection request from a client
    let connection_handler = server
        .incoming()
        .map_err(|InvalidConnection { error, .. }| error)
        .for_each(move |(upgrade, addr)| {
            // Clone again to move into closure "f"
            let connections_inner = connections_inner.clone();
            let entities = entities_inner.clone();
            let counter_inner = counter.clone();
            let executor_2inner = executor_inner.clone();
            let executor_3inner = executor_inner.clone();

            // This future completes the connection and then proceses the sink and stream
            let accept = upgrade
                .accept()
                .and_then(move |(framed, _)| {
                    let (sink, stream) = framed.split();

                    {
                        // Increment the counter by first locking the RwLock
                        let mut c = counter_inner.write().unwrap();
                        *c += 1;
                    }

                    // Assign an id to the new connection and associate with the sink
                    let id = *counter_inner.read().unwrap();
                    connections_inner.write().unwrap().insert(id, sink);

                    // Spawn a stream to process future messages from this client
                    let f = stream
                        .for_each(move |msg| {
                            process_message(&msg, entities.clone());
                            send_state(
                                connections_inner.clone(),
                                entities.clone(),
                                executor_3inner.clone(),
                            );
                            Ok(())
                        })
                        .map_err(|_| ());

                    executor_2inner.spawn(f);

                    println!("user connected: {:?}", id);

                    Ok(())
                })
                .map_err(|_| ());

            executor_inner.spawn(accept);
            Ok(())
        })
        .map_err(|_| ());

    // Finally, block the main thread to wait for the connection_handler stream
    // to finish. Which it never should unless there is an error
    runtime
        .block_on_all(connection_handler)
        .map_err(|_| println!("Error while running core loop"))
        .unwrap();
}

// Send updated state to clients
fn send_state(
    connections: Arc<
        RwLock<HashMap<i32, SplitSink<Framed<TcpStream, MessageCodec<OwnedMessage>>>>>,
    >,
    entities: Arc<RwLock<types::Entities>>,
    executor: TaskExecutor,
) {
    let mut conn = connections.write().unwrap();
    let ids = conn.iter().map(|(k, v)| k.clone()).collect::<Vec<_>>();

    for id in ids.iter() {
        // Must take ownership of the sink to send on it
        // The only way to take ownership of a hashmap value is to remove it
        // And later put it back.
        let sink = conn.remove(id).unwrap();

        // Serialize entitites
        let entities = entities.read().unwrap().clone();
        let serial_entities = serde_json::to_string(&entities).unwrap();

        // Clone for future "f"
        let connections = connections.clone();
        let id = id.clone();

        // This is where the game state is actually sent to the client
        let f = sink
            .send(OwnedMessage::Text(serial_entities))
            .and_then(move |sink| {
                // Re-insert the entry to the connections map
                connections.write().unwrap().insert(id.clone(), sink);
                Ok(())
            })
            .map_err(|_| ());

        executor.spawn(f);
    }
}

// Update entity state with component data
fn process_message(msg: &OwnedMessage, entities: Arc<RwLock<types::Entities>>) {
    if let OwnedMessage::Text(ref txt) = *msg {
        let message: types::Entity = serde_json::from_str(txt).unwrap();

        // Open RW lock
        let mut temp = entities.write().unwrap();
        
        // If no entity by that id create one.
        if !temp.iter().any(|x| x.id == message.id) {
            temp.push(types::Entity {
                id: message.id,
                components: Vec::new(),
            });
        }

        // Get entity index we're working on
        let index = temp.iter().position(|x| x.id == message.id).unwrap();

        // Iterate over component data in our message
        for component in message.components {
            if temp[index].components.iter().any(|x| x.name == component.name) {
                // Has component, overrite 
                let component_index = temp[index].components.iter().position(|x| x.name == component.name).unwrap();
                temp[index].components[component_index] = component;
            } else {
                // Doesn't have component, add it
                temp[index].components.push(component);
            }
        }

        //println!("entities: {:?}", entities);
    }
}
