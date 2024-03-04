use chess::run;

fn main() {
  env_logger::Builder::from_env(
    env_logger::Env::default().default_filter_or("info"),
  )
  .init();
  if let Err(err) = pollster::block_on(run()) {
    log::error!("{:#?}", err);
    // RUST_BACKTRACE=1 to collect
    log::error!("{}", err.backtrace());
  }
}
