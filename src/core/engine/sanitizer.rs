use crate::core::models::workflow::{Workflow, WorkflowEdge, WorkflowNode};
use uuid::Uuid;

/// Đảm bảo tính tương thích của JSON Workflow xuất từ phiên bản cũ (Permissive Studio / Strict Runner).
pub fn sanitize_workflow(mut workflow: Workflow) -> Workflow {
    // 1. Convert legacy `drawflow` to `nodes` and `edges` if missing
    if workflow.nodes.is_none() && workflow.drawflow.is_some() {
        let mut new_nodes = Vec::new();
        let mut new_edges = Vec::new();
        
        if let Some(df) = workflow.drawflow.take() {
            if let Some(nodes) = df.get("nodes").and_then(|n| n.as_array()) {
                for node_val in nodes {
                    let Ok(mut node) = serde_json::from_value::<WorkflowNode>(node_val.clone()) else { continue };
                    if node.id.is_empty() {
                        node.id = Uuid::new_v4().to_string();
                    }
                    if node.r#type.is_none() {
                        node.r#type = Some("BlockBasic".to_string());
                    }
                    new_nodes.push(node);
                }
            }
            if let Some(edges) = df.get("edges").and_then(|e| e.as_array()) {
                for edge_val in edges {
                    let Ok(mut edge) = serde_json::from_value::<WorkflowEdge>(edge_val.clone()) else { continue };
                    if edge.id.is_empty() {
                        edge.id = Uuid::new_v4().to_string();
                    }
                    new_edges.push(edge);
                }
            }
        }
        
        workflow.nodes = Some(new_nodes);
        workflow.edges = Some(new_edges);
    } else {
        // Even if nodes exist, we must ensure they have IDs and Types
        if let Some(ref mut nodes) = workflow.nodes {
            for node in nodes.iter_mut() {
                if node.id.is_empty() {
                    node.id = Uuid::new_v4().to_string();
                }
                if node.r#type.is_none() {
                    node.r#type = Some("BlockBasic".to_string());
                }
            }
        }

        if let Some(ref mut edges) = workflow.edges {
            for edge in edges.iter_mut() {
                if edge.id.is_empty() {
                    edge.id = Uuid::new_v4().to_string();
                }
            }
        }
    }

    workflow
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_missing_ids() {
        let json = r#"{
            "drawflow": {
                "nodes": [
                    { "id": "", "data": {} }
                ],
                "edges": []
            }
        }"#;

        let workflow: Workflow = serde_json::from_str(json).unwrap();
        let sanitized = sanitize_workflow(workflow);

        let nodes = sanitized.nodes.unwrap();
        assert_eq!(nodes.len(), 1);
        assert!(!nodes[0].id.is_empty(), "ID should have been generated");
        assert_eq!(nodes[0].r#type.as_deref().unwrap(), "BlockBasic");
    }

    #[test]
    fn test_sanitize_modern_nodes_without_drawflow() {
        let json = r#"{
            "nodes": [
                { "id": "", "data": { "description": "some node" } },
                { "id": "existing_id", "type": "CustomType" }
            ],
            "edges": [
                { "id": "", "source": "n1", "target": "n2" }
            ]
        }"#;

        let workflow: Workflow = serde_json::from_str(json).unwrap();
        let sanitized = sanitize_workflow(workflow);

        let nodes = sanitized.nodes.unwrap();
        assert_eq!(nodes.len(), 2);
        assert!(!nodes[0].id.is_empty());
        assert_eq!(nodes[0].r#type.as_deref().unwrap(), "BlockBasic");
        assert_eq!(nodes[1].id, "existing_id");
        assert_eq!(nodes[1].r#type.as_deref().unwrap(), "CustomType");

        let edges = sanitized.edges.unwrap();
        assert_eq!(edges.len(), 1);
        assert!(!edges[0].id.is_empty());
    }

    #[test]
    fn test_sanitize_empty_workflow_no_panic() {
        let json = r#"{}"#;
        let workflow: Workflow = serde_json::from_str(json).unwrap();
        let sanitized = sanitize_workflow(workflow);
        assert!(sanitized.nodes.is_none());
        assert!(sanitized.edges.is_none());
    }
}
