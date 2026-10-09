pub fn manage_config(args: &[String], edit: bool, show: bool) -> Result<(), Box<dyn std::error::Error>> {
    crate::config::ConfigController::handle_dispatch("automa", args, edit, show)
}
