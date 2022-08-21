use crate::help_queue::HelpQueue;

use anyhow::Result;
use clap::Parser;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::task::JoinHandle;
use warp::{reject, reply, Filter, Rejection, Reply, hyper::StatusCode};

#[derive(Serialize, Deserialize)]
struct Requester {
    group: u16,
    voice_channel: u64,
}

#[derive(Debug)]
enum ServerError {
    Request(String),
}

impl reject::Reject for ServerError {}

pub trait OrReject<T> {
    fn or_reject(self) -> Result<T, Rejection>;
}

impl<T> OrReject<T> for anyhow::Result<T> {
    fn or_reject(self) -> Result<T, Rejection> {
        self.map_err(|e| reject::custom(ServerError::Request(e.to_string())))
    }
}

#[derive(Debug)]
pub enum HelpQueueRequest {
    Request((u16, u64)),
    Provide(String),
    Dismiss(u16)
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct ServerArguments {
    #[clap(short, long, value_parser, default_value = "http://127.0.0.1")]
    domain: String,
    #[clap(short, long, value_parser, default_value_t = 8080)]
    port: u16,

}

impl Clone for ServerArguments {
    fn clone(&self) -> Self {
        Self { 
            domain: self.domain.clone(), 
            port: self.port
        }
    }
}

impl Default for ServerArguments {
    fn default() -> Self {
        Self {
            domain: "localhost".to_string(),
            port: 8080,
        }
    }
}

fn with<T: Clone + Send>(item: T) -> impl Filter<Extract = (T,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || item.clone())
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct WebServer {
    runtime: tokio::runtime::Runtime,
    args: ServerArguments,
}

impl WebServer {
    pub fn start(help_queue: Arc<HelpQueue>, args: ServerArguments) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_stack_size(8 * 1024 * 1024)
            .build()?;

        let serve_args = args.clone();
        runtime.block_on(async move {
            let _ = Self::start_server(help_queue.clone(), serve_args).await;
        });


        Ok(Self {
            runtime,
            args,
        })
    }

    fn start_server(help_queue: Arc<HelpQueue>, args: ServerArguments) -> JoinHandle<()> {
        let routes = Self::routes(help_queue);
        tokio::spawn(async move {
            println!("\n🌐 Server is running at {}:{}\n", args.domain, args.port);
            warp::serve(routes).run(([127, 0, 0, 1], args.port)).await;
        })
    }

    fn routes(
        help_queue: Arc<HelpQueue>,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        let next = warp::get()
            .and(warp::path!("api" / "discord" / "v1" / "next"))
            .and(warp::body::content_length_limit(64))
            .and(warp::body::json())
            .and(with(help_queue.clone()))
            .and_then(Self::next);

        let dismiss_help = warp::get()
            .and(warp::path!("api" / "discord" / "v1" / "dismiss_help"))
            .and(warp::body::content_length_limit(2))
            .and(warp::body::json())
            .and(with(help_queue.clone()))
            .and_then(Self::dismiss_help);

        let request_help = warp::post()
            .and(warp::path!("api" / "discord" / "v1" / "enqueue_help"))
            .and(warp::body::content_length_limit(10 * 1024 * 1024))
            .and(warp::body::json())
            .and(with(help_queue.clone()))
            .and_then(Self::request_help);

        let clear_queue = warp::patch()
            .and(warp::path!("api" / "discord" / "v1" / "clear_help_queue"))
            .and(with(help_queue.clone()))
            .and_then(Self::clear_help_queue);

        let get_help_queue = warp::get()
            .and(warp::path!("api" / "discord" / "v1" / "help_queue"))
            .and(with(help_queue))
            .and_then(Self::get_help_queue);
        
        next
            .or(dismiss_help)
            .or(request_help)
            .or(clear_queue)
            .or(get_help_queue)
    }

    async fn next(helper: String, help_queue: Arc<HelpQueue>) -> Result<impl Reply, Rejection> {
        let (group, voice_channel) = help_queue.next(helper).await.or_reject()?;
        Ok(reply::with_status(reply::json(&serde_json::json!({"group": group, "voice_channel": voice_channel})), StatusCode::OK))
    }

    async fn dismiss_help(dismisser: u16, help_queue: Arc<HelpQueue>) -> Result<impl Reply, Rejection> {
        let (group, voice_channel) = help_queue.dismiss(dismisser).await.or_reject()?;
        Ok(reply::with_status(reply::json(&serde_json::json!({"group": group, "voice_channel": voice_channel})), StatusCode::OK))
    }

    async fn request_help(requester: Requester, help_queue: Arc<HelpQueue>) -> Result<impl Reply, Rejection> {
        help_queue.enqueue(requester.group, requester.voice_channel).await.or_reject()?;
        Ok(reply::with_status(reply::reply(), StatusCode::OK))
    }

    async fn clear_help_queue(help_queue: Arc<HelpQueue>) -> Result<impl Reply, Rejection> {
        help_queue.clear().await.or_reject()?;
        Ok(reply::with_status(reply::reply(), StatusCode::OK))
    }

    async fn get_help_queue(help_queue: Arc<HelpQueue>) -> Result<impl Reply, Rejection> {
        let queue: Vec<u16> = help_queue.sorted().or_reject()?.collect();
        Ok(reply::with_status(reply::json(&queue), StatusCode::OK))
    }
}
