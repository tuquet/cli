pub fn manage_config(
    args: &[String],
    edit: bool,
    show: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::config::ConfigController::handle_dispatch("automation", args, edit, show)
}
