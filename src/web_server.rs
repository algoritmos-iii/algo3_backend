use crate::help_queue::HelpQueue;

use anyhow::{bail, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::task::JoinHandle;
use warp::{hyper::StatusCode, reject, reply, Filter, Rejection, Reply};

#[derive(Serialize, Deserialize)]
struct Requester {
    group: u16,
    voice_channel: u64,
}

/// An enum of error handlers for the server.
#[derive(Debug)]
enum ServerError {
    Request(String),
}

impl reject::Reject for ServerError {}

/// A trait to unwrap a `Result` or `Reject`.
pub trait OrReject<T> {
    /// Returns the result if it is successful, otherwise returns a rejection.
    fn or_reject(self) -> Result<T, Rejection>;
}

impl<T> OrReject<T> for anyhow::Result<T> {
    /// Returns the result if it is successful, otherwise returns a rejection.
    fn or_reject(self) -> Result<T, Rejection> {
        self.map_err(|e| reject::custom(ServerError::Request(e.to_string())))
    }
}

/// An enum of requests that the `HelpQueue` struct processes.
#[derive(Debug)]
pub enum HelpQueueRequest {
    Request((u16, u64)),
    Provide(String),
    Dismiss(u16),
}

/// Arguments that serve as config for the server.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct ServerArguments {
    #[clap(short, long, value_parser, default_value = "http://0.0.0.0")]
    domain: String,
    #[clap(short, long, value_parser, default_value_t = 80)]
    port: u16,
}

impl Clone for ServerArguments {
    fn clone(&self) -> Self {
        Self {
            domain: self.domain.clone(),
            port: self.port,
        }
    }
}

impl Default for ServerArguments {
    fn default() -> Self {
        Self {
            domain: "http://0.0.0.0".to_string(),
            port: 80,
        }
    }
}

/// A middleware to include the given item in the handler.
fn with<T: Clone + Send>(
    item: T,
) -> impl Filter<Extract = (T,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || item.clone())
}

/// A server for the help queue.
#[allow(dead_code)]
#[derive(Debug)]
pub struct WebServer {
    help_queue: Arc<HelpQueue>,
    runtime: tokio::runtime::Runtime,
    args: ServerArguments,
}

impl WebServer {
    /// Initializes a new instance of the server.
    pub fn start(args: ServerArguments) -> Result<Self> {
        // Initialize a runtime.
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_stack_size(8 * 1024 * 1024)
            .build()?;

        let help_queue = match HelpQueue::new() {
            Ok(help_queue) => help_queue,
            Err(error) => bail!(error.to_string()),
        };

        let serve_args = args.clone();
        let queue = help_queue.clone();
        // Initialize the server.
        runtime.block_on(async move {
            let _ = Self::start_server(queue, serve_args).await;
        });

        Ok(Self {
            help_queue,
            runtime,
            args,
        })
    }

    fn start_server(help_queue: Arc<HelpQueue>, args: ServerArguments) -> JoinHandle<()> {
        // Prepare the list of routes.
        let routes = Self::routes(help_queue);
        tokio::spawn(async move {
            // Start the server.
            println!("\n🌐 Server is running at {}:{}\n", args.domain, args.port);
            warp::serve(routes).run(([0, 0, 0, 0], args.port)).await;
        })
    }

    fn routes(
        help_queue: Arc<HelpQueue>,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        // GET /api/discord/v1/next
        let next = warp::get()
            .and(warp::path!("api" / "discord" / "v1" / "next"))
            .and(warp::body::content_length_limit(64))
            .and(warp::body::json())
            .and(with(help_queue.clone()))
            .and_then(Self::next);

        // POST /api/discord/v1/dismiss_help
        let dismiss_help = warp::get()
            .and(warp::path!("api" / "discord" / "v1" / "dismiss_help"))
            .and(warp::body::content_length_limit(2))
            .and(warp::body::json())
            .and(with(help_queue.clone()))
            .and_then(Self::dismiss_help);

        // POST /api/discord/v1/enqueue_help
        let request_help = warp::post()
            .and(warp::path!("api" / "discord" / "v1" / "enqueue_help"))
            .and(warp::body::content_length_limit(10 * 1024 * 1024))
            .and(warp::body::json())
            .and(with(help_queue.clone()))
            .and_then(Self::request_help);

        // PATCH /api/discord/v1/clear_help_queue
        let clear_queue = warp::patch()
            .and(warp::path!("api" / "discord" / "v1" / "clear_help_queue"))
            .and(with(help_queue.clone()))
            .and_then(Self::clear_help_queue);

        // GET /api/discord/v1/help_queue
        let get_help_queue = warp::get()
            .and(warp::path!("api" / "discord" / "v1" / "help_queue"))
            .and(with(help_queue))
            .and_then(Self::get_help_queue);

        // Return the list of routes.
        next.or(dismiss_help)
            .or(request_help)
            .or(clear_queue)
            .or(get_help_queue)
    }

    /// Returns the next group in the help queue.
    async fn next(helper: String, help_queue: Arc<HelpQueue>) -> Result<impl Reply, Rejection> {
        let (group, voice_channel) = help_queue.next(helper).await.or_reject()?;
        Ok(reply::with_status(
            reply::json(&serde_json::json!({"group": group, "voice_channel": voice_channel})),
            StatusCode::OK,
        ))
    }

    /// Removes the dismisser from the help queue.
    async fn dismiss_help(
        dismisser: u16,
        help_queue: Arc<HelpQueue>,
    ) -> Result<impl Reply, Rejection> {
        let (group, voice_channel) = help_queue.dismiss(dismisser).await.or_reject()?;
        Ok(reply::with_status(
            reply::json(&serde_json::json!({"group": group, "voice_channel": voice_channel})),
            StatusCode::OK,
        ))
    }

    /// Pushes a requester to the help queue.
    async fn request_help(
        requester: Requester,
        help_queue: Arc<HelpQueue>,
    ) -> Result<impl Reply, Rejection> {
        help_queue
            .enqueue(requester.group, requester.voice_channel)
            .await
            .or_reject()?;
        Ok(reply::with_status(reply::reply(), StatusCode::OK))
    }

    /// Clears the help queue.
    async fn clear_help_queue(help_queue: Arc<HelpQueue>) -> Result<impl Reply, Rejection> {
        help_queue.clear().await.or_reject()?;
        Ok(reply::with_status(reply::reply(), StatusCode::OK))
    }

    /// Returns the help queue in order.
    async fn get_help_queue(help_queue: Arc<HelpQueue>) -> Result<impl Reply, Rejection> {
        let queue: Vec<u16> = help_queue.sorted().or_reject()?.collect();
        Ok(reply::with_status(reply::json(&queue), StatusCode::OK))
    }
}
