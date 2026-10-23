pub async fn list_workflows(
    search: Option<String>,
    format: crate::ui::OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    struct WfInfo {
        id: String,
        name: String,
        version: String,
        source: String,
        blocks: usize,
        updated_at: String,
    }

    let mut workflows: Vec<WfInfo> = Vec::new();

    // Workflows come from the Cloud table `automation.workflows` (single source of truth).
    let repo = crate::infrastructure::cloud::CloudWorkflowRepository::connect()
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    for wf in repo.list(search.as_deref()).await? {
        let blocks = wf.block_count();
        workflows.push(WfInfo {
            id: wf.id,
            name: wf.name,
            version: wf.version,
            source: "Cloud".to_string(),
            blocks,
            updated_at: wf.updated_at.unwrap_or_default(),
        });
    }

    let json_workflows: Vec<serde_json::Value> = workflows
        .iter()
        .map(|wf| {
            serde_json::json!({
                "id": wf.id,
                "name": wf.name,
                "version": wf.version,
                "source": wf.source,
                "blocks": wf.blocks,
                "updated_at": wf.updated_at
            })
        })
        .collect();

    crate::ui::respond_with(format, &json_workflows, |_| {
        println!();
        let columns = vec![
            crate::ui::Column {
                title: "ID".to_string(),
                min_width: 18,
                align_right: false,
            },
            crate::ui::Column {
                title: "NAME".to_string(),
                min_width: 24,
                align_right: false,
            },
            crate::ui::Column {
                title: "VERSION".to_string(),
                min_width: 8,
                align_right: false,
            },
            crate::ui::Column {
                title: "SOURCE".to_string(),
                min_width: 10,
                align_right: false,
            },
            crate::ui::Column {
                title: "BLOCKS".to_string(),
                min_width: 6,
                align_right: true,
            },
            crate::ui::Column {
                title: "UPDATED".to_string(),
                min_width: 12,
                align_right: false,
            },
        ];
        let mut table = crate::ui::Table::new(columns);
        for wf in &workflows {
            table.add_row(vec![
                wf.id.clone(),
                wf.name.clone(),
                wf.version.clone(),
                wf.source.clone(),
                wf.blocks.to_string(),
                wf.updated_at.clone(),
            ]);
        }
        if let Some(first) = workflows.first() {
            table = table.with_footer(format!(
                "Run workflow with: tq automation run {} --headless",
                first.id
            ));
        } else {
            table = table.with_footer("Import a workflow with: tq automation workflow import <file.json> --id <workflow_id>");
        }
        table.print();
        println!();
    })
}
