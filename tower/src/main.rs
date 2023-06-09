use axum::{routing::get, Router};
use clap::Parser;
use futures::future;
use std::net::SocketAddr;
use std::{fs, thread};
use tracing::info;
use tracing_subscriber;

use tower_mqtt::MqttServer;
use tower_raft::raft_server;

mod config;
mod flag;
use crate::flag::CliFlag;

fn main() {
    let cli = CliFlag::parse();
    // let t_builder = tracing_subscriber::fmt()
    //     .pretty()
    //     .with_line_number(false)
    //     .with_file(false)
    //     .with_thread_ids(true)
    //     .with_thread_names(true);
    // t_builder.try_init().expect("msg");
    tracing_subscriber::fmt::init();

    let path = cli.config.unwrap();

    info!(
        "config path: {:?}, {:?},{:?}",
        path,
        thread::current().id(),
        thread::current().name()
    );

    let content = fs::read_to_string(path).expect("read config path");
    let config_data: config::Config =
        serde_yaml::from_str(content.as_str()).expect("serialize config failed");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        // .worker_threads(2)
        .build()
        .expect("");

    let raft_config = config_data.peer.clone();
    let raft_handle = runtime.spawn(async move {
        let addr: SocketAddr = raft_config
            .server_addr
            .parse()
            .expect("address is not valid");
        let raft = raft_server::RaftServer::new(addr);
        raft.run().await;
    });

    let mq_handle = runtime.spawn(async {
        let s = MqttServer::new();
        s.run().await;
    });

    let api_handle = runtime.spawn(async {
        let app = Router::new().route("/", get(|| async { "Hello, World!" }));
        info!("api service started in: {}", "0.0.0.0:3000");
        axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    let servers = vec![raft_handle, mq_handle, api_handle];

    runtime.block_on(async {
        future::join_all(servers).await;
    });
}
