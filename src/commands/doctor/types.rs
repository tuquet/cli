pub struct DiagnosticItem {
    pub name: String,
    pub category: &'static str,
    pub ok: bool,
    pub status_text: String,
    pub path_or_detail: String,
    pub action_hint: Option<String>,
}
