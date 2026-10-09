pub mod automa;
pub mod browser;
pub mod bridge;
pub mod cloud;
pub mod faker;
pub mod global;
pub mod runner;

pub use automa::dispatch_automa;
pub use browser::dispatch_browser;
pub use bridge::dispatch_bridge;
pub use cloud::dispatch_cloud;
pub use faker::dispatch_faker;
pub use global::dispatch_global;
pub use runner::dispatch_runner;

pub fn get_flag_value<'a>(args: &'a [&'a str], short: &str, long: &str) -> Option<&'a str> {
    args.windows(2).find(|w| w[0] == short || w[0] == long).map(|w| w[1])
}
