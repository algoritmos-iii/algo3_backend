use algo3_backend::{web_server::ServerArguments, help_queue::HelpQueue};

use clap::Parser;

fn main() {
    match WebServer::start(ServerArguments::parse()) {
        Ok(_) => {},
        Err(error) => eprintln!("Error al correr el servidor: {}", error)
    }
}
