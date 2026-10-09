pub mod config;
pub mod executor;
pub mod inspector;
pub mod studio;
pub mod table;
pub mod vault;

pub use config::manage_config;
pub use executor::run_workflow;
pub use inspector::inspect_workflow;
pub use studio::open_studio;
pub use table::list_workflows;
pub use vault::{delete_workflow, export_workflow, import_workflow};

use crate::cli::{AutomaSubcommands, WorkflowCommands};

pub async fn handle(command: AutomaSubcommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        AutomaSubcommands::Run {
            workflow_pos,
            workflow,
            workflow_json,
            headless,
            browser,
            browser_id,
            variables,
            timeout,
            cloud_profile,
        } => {
            let target_workflow = workflow.or(workflow_pos);
            if let Some(ref target_cloud) = cloud_profile {
                crate::infrastructure::cloud_fleet::CloudFleetOrchestrator::run_autonomous_cloud_session(
                    target_cloud,
                    target_workflow,
                    workflow_json,
                    headless,
                    variables,
                    timeout,
                )
                .await
            } else {
                run_workflow(
                    target_workflow,
                    workflow_json,
                    headless,
                    browser,
                    browser_id,
                    variables,
                    timeout,
                )
                .await
            }
        }
        AutomaSubcommands::Workflow { command } => match command {
            WorkflowCommands::List {
                search,
                db_only,
                vault_only,
                format,
            } => list_workflows(search, db_only, vault_only, format.resolve()).await,
            WorkflowCommands::Import {
                file,
                id,
                name,
                description,
            } => import_workflow(file, id, name, description).await,
            WorkflowCommands::Export { id, output } => export_workflow(id, output).await,
            WorkflowCommands::Info { id } => inspect_workflow(&id),
            WorkflowCommands::Delete { id, vault } => delete_workflow(id, vault).await,
        },
        AutomaSubcommands::Inspect { workflow } => inspect_workflow(&workflow),
        AutomaSubcommands::Studio => open_studio(),
        AutomaSubcommands::Probe => crate::commands::runner::print_probe_manifest(),
        AutomaSubcommands::Config { edit, show, args } => manage_config(&args, edit, show),
    }
}
