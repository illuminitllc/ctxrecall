use std::path::Path;

use crate::db::document_repo::Document;
use crate::tracker::types::Issue;

const TOTAL_CAP: usize = 12_000;
const DESC_CAP: usize = 2_000;
const SUMMARY_CAP: usize = 1_500;
const DOC_CAP: usize = 2_000;
const PROJECT_DOC_CAP: usize = 1_500;
const MAX_SUMMARIES: usize = 3;

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    // Find a char boundary at or before max
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Load `.md` and `.txt` files from a project's `docs/` directory.
/// Returns (filename, content) pairs.
pub fn load_project_docs(project_dir: &Path) -> Vec<(String, String)> {
    let docs_dir = project_dir.join("docs");
    let Ok(entries) = std::fs::read_dir(&docs_dir) else {
        return Vec::new();
    };

    let mut results = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "md" && ext != "txt" {
            continue;
        }
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if let Ok(content) = std::fs::read_to_string(&path) {
            if !content.trim().is_empty() {
                results.push((filename, content));
            }
        }
    }
    results.sort_by(|a, b| a.0.cmp(&b.0));
    results
}

/// Build a context prompt from issue data, transcript summaries, linked documents,
/// and project-level docs. Returns `None` if there's no meaningful content to inject.
pub fn build_context_prompt(
    issue: &Issue,
    summaries: &[String],
    documents: &[Document],
    project_docs: &[(String, String)],
    branch: Option<&str>,
) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();

    // Issue metadata
    let mut meta = format!(
        "# Issue: {} — {}\n**Status:** {}",
        issue.identifier, issue.title, issue.status
    );
    if let Some(team) = &issue.team {
        meta.push_str(&format!(" | **Team:** {team}"));
    }
    if let Some(project) = &issue.project {
        meta.push_str(&format!(" | **Project:** {project}"));
    }
    if let Some(assignee) = &issue.assignee {
        meta.push_str(&format!(" | **Assignee:** {assignee}"));
    }
    if !issue.labels.is_empty() {
        meta.push_str(&format!(" | **Labels:** {}", issue.labels.join(", ")));
    }
    if let Some(branch_name) = branch {
        meta.push_str(&format!(" | **Branch:** {branch_name}"));
    }
    parts.push(meta);

    // Issue description
    if let Some(desc) = &issue.description {
        let trimmed = desc.trim();
        if !trimmed.is_empty() {
            let desc_text = truncate(trimmed, DESC_CAP);
            parts.push(format!("## Description\n{desc_text}"));
        }
    }

    // Transcript summaries (newest first, already sorted by caller)
    let relevant_summaries: Vec<&String> = summaries
        .iter()
        .filter(|s| !s.trim().is_empty())
        .take(MAX_SUMMARIES)
        .collect();
    if !relevant_summaries.is_empty() {
        let mut section = String::from("## Previous Session Summaries");
        for (i, summary) in relevant_summaries.iter().enumerate() {
            let text = truncate(summary.trim(), SUMMARY_CAP);
            section.push_str(&format!("\n### Session {}\n{text}", i + 1));
        }
        parts.push(section);
    }

    // Linked documents
    let relevant_docs: Vec<&Document> = documents
        .iter()
        .filter(|d| !d.content.trim().is_empty())
        .collect();
    if !relevant_docs.is_empty() {
        let mut section = String::from("## Linked Documents");
        for doc in &relevant_docs {
            let text = truncate(doc.content.trim(), DOC_CAP);
            section.push_str(&format!("\n### {} ({})\n{text}", doc.title, doc.doc_type));
        }
        parts.push(section);
    }

    // Project-level docs (from filesystem), deduped against DB docs
    let db_basenames: Vec<String> = documents
        .iter()
        .filter_map(|d| {
            d.file_path.as_ref().and_then(|fp| {
                Path::new(fp)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            })
        })
        .collect();
    let unique_project_docs: Vec<&(String, String)> = project_docs
        .iter()
        .filter(|(name, _)| !db_basenames.contains(name))
        .collect();
    if !unique_project_docs.is_empty() {
        let mut section = String::from("## Project Documents");
        for (name, content) in &unique_project_docs {
            let text = truncate(content.trim(), PROJECT_DOC_CAP);
            section.push_str(&format!("\n### {name}\n{text}"));
        }
        parts.push(section);
    }

    if let Some(branch_name) = branch {
        parts.push(format!(
            "## Git Branch\nThis issue is tracked on branch `{branch_name}`. \
             Please verify you are on this branch before making changes. \
             If not, switch to it with `git checkout {branch_name}`."
        ));
    }

    // Nothing beyond the identifier/title? Skip injection.
    if parts.len() <= 1 && issue.description.as_ref().map_or(true, |d| d.trim().is_empty()) {
        return None;
    }

    parts.push(String::from(
        "---\nPlease review this context and confirm you're ready to work on this issue.",
    ));

    let mut prompt = parts.join("\n\n");

    // Enforce total cap
    if prompt.len() > TOTAL_CAP {
        let end = {
            let mut e = TOTAL_CAP;
            while e > 0 && !prompt.is_char_boundary(e) {
                e -= 1;
            }
            e
        };
        prompt.truncate(end);
    }

    Some(prompt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn test_issue() -> Issue {
        Issue {
            id: "id-1".into(),
            identifier: "ENG-42".into(),
            title: "Fix the widget".into(),
            description: Some("The widget is broken and needs fixing.".into()),
            status: "In Progress".into(),
            status_id: None,
            priority: 2,
            assignee: Some("Alice".into()),
            assignee_id: None,
            team: Some("Engineering".into()),
            team_id: None,
            project: Some("Backend".into()),
            project_id: None,
            labels: vec!["bug".into(), "urgent".into()],
            url: "https://example.com".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn builds_prompt_with_description() {
        let issue = test_issue();
        let result = build_context_prompt(&issue, &[], &[], &[], None);
        assert!(result.is_some());
        let prompt = result.unwrap();
        assert!(prompt.contains("ENG-42"));
        assert!(prompt.contains("Fix the widget"));
        assert!(prompt.contains("broken and needs fixing"));
        assert!(prompt.contains("confirm you're ready"));
    }

    #[test]
    fn returns_none_for_empty_issue() {
        let mut issue = test_issue();
        issue.description = None;
        let result = build_context_prompt(&issue, &[], &[], &[], None);
        assert!(result.is_none());
    }

    #[test]
    fn includes_summaries_and_docs() {
        let issue = test_issue();
        let summaries = vec!["Summary of session 1".into(), "Summary of session 2".into()];
        let docs = vec![Document {
            id: "d1".into(),
            issue_id: "id-1".into(),
            doc_type: "spec".into(),
            title: "Design Spec".into(),
            content: "Detailed spec content".into(),
            created_at: String::new(),
            updated_at: String::new(),
            file_path: None,
        }];
        let result = build_context_prompt(&issue, &summaries, &docs, &[], None);
        let prompt = result.unwrap();
        assert!(prompt.contains("Session 1"));
        assert!(prompt.contains("Summary of session 1"));
        assert!(prompt.contains("Design Spec"));
    }
}
