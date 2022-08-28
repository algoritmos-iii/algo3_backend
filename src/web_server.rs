use crate::{
    google_services::{GoogleService, ServiceType, SpreadsheetService},
    help_queue::HelpQueue,
};

use anyhow::{bail, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::task::JoinHandle;
use warp::{hyper::StatusCode, reject, reply, Filter, Rejection, Reply};

#[derive(Serialize, Deserialize)]
struct Requester {
    group: u16,
    voice_channel: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Student {
    pub id: u32,
    pub email: String,
}

/// An enum of error handlers for the server.
#[derive(Debug)]
enum ServerError {
    Request(String),
    StudentNotFound,
    InvalidStudentId,
    InvalidStudentEmail,
    InvalidGroup,
    StudentHasNoGroup,
    GoogleServiceResponse,
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
    #[clap(
        short,
        long,
        value_parser,
        default_value = "1jWXRFLamVmuAyTpv-n6737ze-8sgoAv1ZzHdyFXn4Rg"
    )]
    spreadsheet_id: String,
    #[clap(
        short,
        long,
        value_parser,
        default_value = "1Ss7FMebxZxxGi15mREvQLYuBJ1sWVWbD"
    )]
    helpsheet_id: String,
}

impl Clone for ServerArguments {
    fn clone(&self) -> Self {
        Self {
            domain: self.domain.clone(),
            port: self.port,
            spreadsheet_id: self.spreadsheet_id.clone(),
            helpsheet_id: self.helpsheet_id.clone(),
        }
    }
}

impl Default for ServerArguments {
    fn default() -> Self {
        Self {
            domain: "http://0.0.0.0".to_string(),
            port: 80,
            spreadsheet_id: "1jWXRFLamVmuAyTpv-n6737ze-8sgoAv1ZzHdyFXn4Rg".to_string(),
            helpsheet_id: "1Ss7FMebxZxxGi15mREvQLYuBJ1sWVWbD".to_string(),
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
        tokio::spawn(async move {
            let spreadsheet_service = SpreadsheetService::new_reading_service_account_key(
                ServiceType::Spreadsheet,
                "v4",
                "./clientsecret.json",
                &["https://www.googleapis.com/auth/spreadsheets"],
            )
            .await
            .unwrap();
            let routes = Self::routes(help_queue, spreadsheet_service, args.clone());
            // Start the server.
            println!("\n🌐 Server is running at {}:{}\n", args.domain, args.port);
            warp::serve(routes).run(([0, 0, 0, 0], args.port)).await;
        })
    }

    fn routes(
        help_queue: Arc<HelpQueue>,
        spreadsheet_service: Arc<SpreadsheetService>,
        args: ServerArguments,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        // GET /api/discord/v1/next
        let next = warp::get()
            .and(warp::path!("api" / "discord" / "v1" / "next"))
            .and(warp::body::content_length_limit(64))
            .and(warp::body::json())
            .and(with(spreadsheet_service.clone()))
            .and(with(args.helpsheet_id.clone()))
            .and(with(help_queue.clone()))
            .and_then(Self::next);

        // POST /api/discord/v1/dismiss_help
        let dismiss_help = warp::get()
            .and(warp::path!("api" / "discord" / "v1" / "dismiss_help"))
            .and(warp::body::content_length_limit(2))
            .and(warp::body::json())
            .and(with(spreadsheet_service.clone()))
            .and(with(args.helpsheet_id.clone()))
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

        let is_student = warp::get()
            .and(warp::path!("api" / "discord" / "v1" / "is_student"))
            .and(warp::body::content_length_limit(10 * 1024 * 1024))
            .and(warp::body::json())
            .and(with(spreadsheet_service.clone()))
            .and(with(args.spreadsheet_id.clone()))
            .and_then(Self::is_student);

        let get_group = warp::get()
            .and(warp::path!("api" / "discord" / "v1" / "group"))
            .and(warp::body::content_length_limit(10 * 1024 * 1024))
            .and(warp::body::json())
            .and(with(spreadsheet_service))
            .and(with(args.spreadsheet_id))
            .and_then(Self::find_group);

        // Return the list of routes.
        next.or(dismiss_help)
            .or(request_help)
            .or(clear_queue)
            .or(get_help_queue)
            .or(is_student)
            .or(get_group)
    }

    /// Returns the next group in the help queue.
    async fn next(
        helper: String,
        spreadsheet_service: Arc<SpreadsheetService>,
        helpsheet_id: String,
        help_queue: Arc<HelpQueue>,
    ) -> Result<impl Reply, Rejection> {
        let (group, voice_channel) = help_queue.next(&helper).await.or_reject()?;
        Self::log_help(
            group,
            "Brindada",
            &helper,
            spreadsheet_service,
            helpsheet_id,
        )
        .await?;
        Ok(reply::with_status(
            reply::json(&serde_json::json!({"group": group, "voice_channel": voice_channel})),
            StatusCode::OK,
        ))
    }

    /// Removes the dismisser from the help queue.
    async fn dismiss_help(
        dismisser: u16,
        spreadsheet_service: Arc<SpreadsheetService>,
        helpsheet_id: String,
        help_queue: Arc<HelpQueue>,
    ) -> Result<impl Reply, Rejection> {
        let (group, voice_channel) = help_queue.dismiss(dismisser).await.or_reject()?;
        Self::log_help(group, "Desestimada", "-", spreadsheet_service, helpsheet_id).await?;
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

    async fn is_student(
        student_model: Student,
        service: Arc<SpreadsheetService>,
        spreadsheet_id: String,
    ) -> Result<impl Reply, Rejection> {
        let student_id = student_model.id;
        let student_email = student_model.email;

        // TODO: validate student model fields.

        let student_column_range = "B:E";
        let sheet_id = "Listado";

        let spreadsheet_values = service
            .get_values(&spreadsheet_id, sheet_id, student_column_range)
            .await
            .or_reject()?;

        let is_student = spreadsheet_values.values.into_iter().any(|mut row| {
            row.splice(1..3, vec![]);
            row.contains(&student_id.to_string()) && row.contains(&student_email)
        });

        Ok(reply::with_status(
            reply::json(&json!(is_student)),
            StatusCode::OK,
        ))

        // TODO: Update "Está en Discord" column (K).
    }

    pub async fn find_group(
        student_model: Student,
        service: Arc<SpreadsheetService>,
        spreadsheet_id: String,
    ) -> Result<impl Reply, Rejection> {
        let student_id = student_model.id;
        let student_email = student_model.email;

        // TODO: validate student model fields.

        println!("Finding group of: {student_id} {student_email}");

        let spreadsheet_values = service
            .get_values(&spreadsheet_id, "Listado", "B:E")
            .await
            .or_reject()?;

        println!("Spreadsheet retrieved successfully.");

        let plausible_student = spreadsheet_values
            .values
            .into_iter()
            .map(|mut row| {
                row.splice(1..2, vec![]);
                row
            })
            .find(|row| row[0] == student_id.to_string() && row[2] == student_email);

        let student = match plausible_student {
            Some(student) => student,
            None => return Err(reject::custom(ServerError::StudentNotFound)),
        };

        println!("Student found.");

        let group = match student[1].parse::<u8>() {
            Ok(group) => group,
            Err(_) => return Err(reject::custom(ServerError::StudentHasNoGroup)),
        };

        println!("Student group: {group} for {student_id}.");

        Ok(reply::with_status(
            reply::json(&json!(group)),
            StatusCode::OK,
        ))
    }

    pub async fn log_help(
        group: u16,
        resolution: &str,
        helper: &str,
        service: Arc<SpreadsheetService>,
        helpsheet_id: String,
    ) -> Result<impl Reply, Rejection> {
        println!("Logging help");

        let iso_date = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let sheet_id = "Ayudas";
        let plausible_response = service
            .append_row(
                &helpsheet_id,
                sheet_id,
                vec![&iso_date, &group.to_string(), resolution, helper],
            )
            .await;

        match plausible_response {
            Ok(response) => {
                println!("Help logged");
                Ok(warp::reply::with_status(
                    warp::reply::json(&response.json::<serde_json::Value>().await.unwrap()),
                    warp::http::StatusCode::OK,
                ))
            }
            Err(_) => Err(reject::not_found()),
        }
    }
}
