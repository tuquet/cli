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

pub fn parse_format(args: &[&str]) -> crate::ui::OutputFormat {
    if args.contains(&"--card") || args.contains(&"-c") {
        crate::ui::OutputFormat::Card
    } else if args.contains(&"--table") {
        crate::ui::OutputFormat::Table
    } else if args.contains(&"--json") || args.contains(&"-j") {
        crate::ui::OutputFormat::Json
    } else if let Some(val) = get_flag_value(args, "-f", "--format") {
        match val.trim().to_lowercase().as_str() {
            "card" => crate::ui::OutputFormat::Card,
            "table" => crate::ui::OutputFormat::Table,
            _ => crate::ui::OutputFormat::Json,
        }
    } else {
        crate::ui::OutputFormat::Json
    }
}

