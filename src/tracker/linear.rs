use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use super::IssueTracker;
use super::types::{Issue, IssueFilter, IssueStatus, IssueUpdate, Label, NewIssue, Project, Team};
use crate::errors::AppError;

const LINEAR_API_URL: &str = "https://api.linear.app/graphql";

pub struct LinearTracker {
    client: Client,
    api_key: String,
}

// GraphQL response types for deserialization
#[derive(Deserialize)]
struct GqlResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GqlError>>,
}

#[derive(Deserialize)]
struct GqlError {
    message: String,
}

#[derive(Deserialize)]
struct IssuesData {
    issues: IssueConnection,
}

#[derive(Deserialize)]
struct IssueConnection {
    nodes: Vec<GqlIssue>,
}

#[derive(Deserialize)]
struct SingleIssueData {
    issue: GqlIssue,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlIssue {
    id: String,
    identifier: String,
    title: String,
    description: Option<String>,
    priority: i32,
    url: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    state: GqlState,
    assignee: Option<GqlUser>,
    team: GqlTeamRef,
    project: Option<GqlProjectRef>,
    labels: GqlLabelConnection,
}

#[derive(Deserialize)]
struct GqlState {
    id: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlUser {
    id: String,
    display_name: String,
}

#[derive(Deserialize)]
struct GqlTeamRef {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct GqlProjectRef {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct GqlLabelConnection {
    nodes: Vec<GqlLabel>,
}

#[derive(Deserialize)]
struct GqlLabel {
    name: String,
}

#[derive(Deserialize)]
struct TeamsData {
    teams: TeamConnection,
}

#[derive(Deserialize)]
struct TeamConnection {
    nodes: Vec<GqlTeam>,
}

#[derive(Deserialize)]
struct GqlTeam {
    id: String,
    name: String,
    key: String,
}

#[derive(Deserialize)]
struct ProjectsData {
    projects: ProjectConnection,
}

#[derive(Deserialize)]
struct ProjectConnection {
    nodes: Vec<GqlProject>,
}

#[derive(Deserialize)]
struct GqlProject {
    id: String,
    name: String,
    teams: TeamConnection,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowStatesData {
    workflow_states: WorkflowStateConnection,
}

#[derive(Deserialize)]
struct WorkflowStateConnection {
    nodes: Vec<GqlWorkflowState>,
}

#[derive(Deserialize)]
struct GqlWorkflowState {
    id: String,
    name: String,
    color: String,
    position: f64,
    team: GqlWorkflowTeamRef,
}

#[derive(Deserialize)]
struct GqlWorkflowTeamRef {
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LabelsData {
    issue_labels: IssueLabelConnection,
}

#[derive(Deserialize)]
struct IssueLabelConnection {
    nodes: Vec<GqlIssueLabel>,
}

#[derive(Deserialize)]
struct GqlIssueLabel {
    id: String,
    name: String,
    color: String,
    team: Option<GqlWorkflowTeamRef>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssueUpdateData {
    issue_update: IssueUpdatePayload,
}

#[derive(Deserialize)]
struct IssueUpdatePayload {
    issue: GqlIssue,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssueCreateData {
    issue_create: IssueCreatePayload,
}

#[derive(Deserialize)]
struct IssueCreatePayload {
    issue: GqlIssue,
}

impl LinearTracker {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    async fn query<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<T, AppError> {
        let body = json!({
            "query": query,
            "variables": variables,
        });

        let resp = self
            .client
            .post(LINEAR_API_URL)
            .header("Authorization", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Api(format!("HTTP error: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp
                .text()
                .await
                .unwrap_or_else(|_| "unknown".to_string());
            return Err(AppError::Api(format!("HTTP {status}: {text}")));
        }

        let gql_resp: GqlResponse<T> = resp
            .json()
            .await
            .map_err(|e| AppError::Api(format!("JSON parse error: {e}")))?;

        if let Some(errors) = gql_resp.errors {
            let msgs: Vec<String> = errors.into_iter().map(|e| e.message).collect();
            return Err(AppError::Api(msgs.join("; ")));
        }

        gql_resp
            .data
            .ok_or_else(|| AppError::Api("No data in response".into()))
    }

    fn build_issue_filter(filter: &IssueFilter) -> serde_json::Value {
        let mut f = serde_json::Map::new();

        if let Some(team_id) = &filter.team_id {
            f.insert(
                "team".into(),
                json!({ "id": { "eq": team_id } }),
            );
        }
        if let Some(project_id) = &filter.project_id {
            f.insert(
                "project".into(),
                json!({ "id": { "eq": project_id } }),
            );
        }
        if let Some(status) = &filter.status {
            f.insert(
                "state".into(),
                json!({ "name": { "eq": status } }),
            );
        }
        if let Some(assignee) = &filter.assignee {
            f.insert(
                "assignee".into(),
                json!({ "displayName": { "eq": assignee } }),
            );
        }

        serde_json::Value::Object(f)
    }
}

impl From<GqlIssue> for Issue {
    fn from(g: GqlIssue) -> Self {
        Issue {
            id: g.id,
            identifier: g.identifier,
            title: g.title,
            description: g.description,
            status: g.state.name,
            status_id: Some(g.state.id),
            priority: g.priority,
            assignee_id: g.assignee.as_ref().map(|u| u.id.clone()),
            assignee: g.assignee.map(|u| u.display_name),
            team: Some(g.team.name),
            team_id: Some(g.team.id),
            project: g.project.as_ref().map(|p| p.name.clone()),
            project_id: g.project.map(|p| p.id),
            labels: g.labels.nodes.into_iter().map(|l| l.name).collect(),
            url: g.url,
            created_at: g.created_at,
            updated_at: g.updated_at,
        }
    }
}

const ISSUES_QUERY: &str = r#"
query ListIssues($filter: IssueFilter, $first: Int) {
    issues(filter: $filter, first: $first, orderBy: updatedAt) {
        nodes {
            id
            identifier
            title
            description
            priority
            url
            createdAt
            updatedAt
            state { id name }
            assignee { id displayName }
            team { id name }
            project { id name }
            labels { nodes { name } }
        }
    }
}
"#;

const ISSUE_QUERY: &str = r#"
query GetIssue($id: String!) {
    issue(id: $id) {
        id
        identifier
        title
        description
        priority
        url
        createdAt
        updatedAt
        state { id name }
        assignee { displayName }
        team { id name }
        project { id name }
        labels { nodes { name } }
    }
}
"#;

const TEAMS_QUERY: &str = r#"
query ListTeams {
    teams {
        nodes {
            id
            name
            key
        }
    }
}
"#;

const UPDATE_ISSUE_MUTATION: &str = r#"
mutation UpdateIssue($id: String!, $input: IssueUpdateInput!) {
    issueUpdate(id: $id, input: $input) {
        issue {
            id
            identifier
            title
            description
            priority
            url
            createdAt
            updatedAt
            state { id name }
            assignee { id displayName }
            team { id name }
            project { id name }
            labels { nodes { name } }
        }
    }
}
"#;

const CREATE_ISSUE_MUTATION: &str = r#"
mutation CreateIssue($input: IssueCreateInput!) {
    issueCreate(input: $input) {
        issue {
            id
            identifier
            title
            description
            priority
            url
            createdAt
            updatedAt
            state { id name }
            assignee { id displayName }
            team { id name }
            project { id name }
            labels { nodes { name } }
        }
    }
}
"#;

const PROJECTS_QUERY: &str = r#"
query ListProjects {
    projects {
        nodes {
            id
            name
            teams {
                nodes {
                    id
                    name
                    key
                }
            }
        }
    }
}
"#;

const LABELS_QUERY: &str = r#"
query ListLabels {
    issueLabels(first: 200) {
        nodes { id name color team { id } }
    }
}
"#;

const WORKFLOW_STATES_QUERY: &str = r#"
query ListWorkflowStates {
    workflowStates(first: 200) {
        nodes {
            id
            name
            color
            position
            team { id }
        }
    }
}
"#;

#[async_trait]
impl IssueTracker for LinearTracker {
    async fn list_issues(&self, filter: &IssueFilter) -> Result<Vec<Issue>, AppError> {
        let filter_value = Self::build_issue_filter(filter);
        let variables = if filter_value.as_object().unwrap().is_empty() {
            json!({ "first": 50 })
        } else {
            json!({ "filter": filter_value, "first": 50 })
        };

        let data: IssuesData = self.query(ISSUES_QUERY, variables).await?;
        Ok(data.issues.nodes.into_iter().map(Issue::from).collect())
    }

    async fn get_issue(&self, id: &str) -> Result<Issue, AppError> {
        let data: SingleIssueData = self
            .query(ISSUE_QUERY, json!({ "id": id }))
            .await?;
        Ok(Issue::from(data.issue))
    }

    async fn update_issue(&self, id: &str, update: &IssueUpdate) -> Result<Issue, AppError> {
        let mut input = serde_json::Map::new();
        if let Some(title) = &update.title {
            input.insert("title".into(), json!(title));
        }
        if let Some(desc) = &update.description {
            input.insert("description".into(), json!(desc));
        }
        if let Some(status_id) = &update.status_id {
            input.insert("stateId".into(), json!(status_id));
        }
        if let Some(assignee_id) = &update.assignee_id {
            input.insert("assigneeId".into(), json!(assignee_id));
        }
        if let Some(priority) = &update.priority {
            input.insert("priority".into(), json!(priority));
        }
        if let Some(label_ids) = &update.label_ids {
            input.insert("labelIds".into(), json!(label_ids));
        }

        let data: IssueUpdateData = self
            .query(
                UPDATE_ISSUE_MUTATION,
                json!({ "id": id, "input": serde_json::Value::Object(input) }),
            )
            .await?;
        Ok(Issue::from(data.issue_update.issue))
    }

    async fn create_issue(&self, new_issue: &NewIssue) -> Result<Issue, AppError> {
        let mut input = serde_json::Map::new();
        input.insert("title".into(), json!(new_issue.title));
        input.insert("teamId".into(), json!(new_issue.team_id));
        if let Some(desc) = &new_issue.description {
            input.insert("description".into(), json!(desc));
        }
        if let Some(project_id) = &new_issue.project_id {
            input.insert("projectId".into(), json!(project_id));
        }
        if let Some(priority) = &new_issue.priority {
            input.insert("priority".into(), json!(priority));
        }
        if let Some(assignee_id) = &new_issue.assignee_id {
            input.insert("assigneeId".into(), json!(assignee_id));
        }
        if let Some(label_ids) = &new_issue.label_ids {
            input.insert("labelIds".into(), json!(label_ids));
        }

        let data: IssueCreateData = self
            .query(
                CREATE_ISSUE_MUTATION,
                json!({ "input": serde_json::Value::Object(input) }),
            )
            .await?;
        Ok(Issue::from(data.issue_create.issue))
    }

    async fn list_teams(&self) -> Result<Vec<Team>, AppError> {
        let data: TeamsData = self.query(TEAMS_QUERY, json!({})).await?;
        Ok(data
            .teams
            .nodes
            .into_iter()
            .map(|t| Team {
                id: t.id,
                name: t.name,
                key: t.key,
            })
            .collect())
    }

    async fn list_projects(&self) -> Result<Vec<Project>, AppError> {
        let data: ProjectsData = self.query(PROJECTS_QUERY, json!({})).await?;
        Ok(data
            .projects
            .nodes
            .into_iter()
            .map(|p| Project {
                id: p.id,
                name: p.name,
                team_ids: p.teams.nodes.into_iter().map(|t| t.id).collect(),
            })
            .collect())
    }

    async fn list_workflow_states(&self) -> Result<Vec<IssueStatus>, AppError> {
        let data: WorkflowStatesData = self
            .query(WORKFLOW_STATES_QUERY, json!({}))
            .await?;
        Ok(data
            .workflow_states
            .nodes
            .into_iter()
            .map(|s| IssueStatus {
                id: s.id,
                name: s.name,
                team_id: s.team.id,
                color: s.color,
                position: s.position,
            })
            .collect())
    }

    async fn list_labels(&self) -> Result<Vec<Label>, AppError> {
        let data: LabelsData = self.query(LABELS_QUERY, json!({})).await?;
        Ok(data
            .issue_labels
            .nodes
            .into_iter()
            .map(|l| Label {
                id: l.id,
                name: l.name,
                color: l.color,
                team_id: l.team.map(|t| t.id),
            })
            .collect())
    }
}
