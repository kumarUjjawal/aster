mod app;
mod error;
mod logging;
mod model;
mod services;
mod ui;

fn main() {
    logging::init_tracing();
    app::run();
}
