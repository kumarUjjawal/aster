use tracing_subscriber::fmt;

pub fn init_tracing() {
    let _ = fmt().with_target(false).compact().try_init();
}
