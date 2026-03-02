use rusqlite::Connection;

use crate::errors::AppError;
use crate::tracker::types::{Issue, IssueStatus, Label, Team, Project};

pub fn upsert_issues(conn: &Connection, issues: &[Issue]) -> Result<(), AppError> {
    let mut stmt = conn.prepare(
        "INSERT INTO issues (id, identifier, title, description, status, status_id, priority, assignee, assignee_id, team, team_id, project, project_id, labels, url, created_at, updated_at, cached_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
            identifier = ?2, title = ?3, description = ?4, status = ?5, status_id = ?6,
            priority = ?7, assignee = ?8, assignee_id = ?9, team = ?10, team_id = ?11, project = ?12, project_id = ?13,
            labels = ?14, url = ?15, created_at = ?16, updated_at = ?17,
            cached_at = datetime('now')"
    )?;

    for issue in issues {
        let labels_json = serde_json::to_string(&issue.labels).unwrap_or_default();
        stmt.execute(rusqlite::params![
            issue.id,
            issue.identifier,
            issue.title,
            issue.description,
            issue.status,
            issue.status_id,
            issue.priority,
            issue.assignee,
            issue.assignee_id,
            issue.team,
            issue.team_id,
            issue.project,
            issue.project_id,
            labels_json,
            issue.url,
            issue.created_at.to_rfc3339(),
            issue.updated_at.to_rfc3339(),
        ])?;
    }

    Ok(())
}

pub fn load_cached_issues(conn: &Connection) -> Result<Vec<Issue>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, identifier, title, description, status, status_id, priority, assignee, assignee_id, team, team_id, project, project_id, labels, url, created_at, updated_at
         FROM issues ORDER BY updated_at DESC"
    )?;

    let issues = stmt.query_map([], |row| {
        let labels_str: String = row.get(13)?;
        let labels: Vec<String> = serde_json::from_str(&labels_str).unwrap_or_default();
        let created_str: String = row.get(15)?;
        let updated_str: String = row.get(16)?;

        Ok(Issue {
            id: row.get(0)?,
            identifier: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            status: row.get(4)?,
            status_id: row.get(5)?,
            priority: row.get(6)?,
            assignee: row.get(7)?,
            assignee_id: row.get(8)?,
            team: row.get(9)?,
            team_id: row.get(10)?,
            project: row.get(11)?,
            project_id: row.get(12)?,
            labels,
            url: row.get(14)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_default(),
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_default(),
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(issues)
}

pub fn upsert_teams(conn: &Connection, teams: &[Team]) -> Result<(), AppError> {
    let mut stmt = conn.prepare(
        "INSERT INTO teams (id, name, key, cached_at)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET name = ?2, key = ?3, cached_at = datetime('now')"
    )?;

    for team in teams {
        stmt.execute(rusqlite::params![team.id, team.name, team.key])?;
    }

    Ok(())
}

pub fn load_cached_teams(conn: &Connection) -> Result<Vec<Team>, AppError> {
    let mut stmt = conn.prepare("SELECT id, name, key FROM teams ORDER BY name")?;
    let teams = stmt.query_map([], |row| {
        Ok(Team {
            id: row.get(0)?,
            name: row.get(1)?,
            key: row.get(2)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(teams)
}

pub fn upsert_projects(conn: &Connection, projects: &[Project]) -> Result<(), AppError> {
    let mut stmt = conn.prepare(
        "INSERT INTO projects (id, name, team_ids, cached_at)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET name = ?2, team_ids = ?3, cached_at = datetime('now')"
    )?;

    for project in projects {
        let team_ids_json = serde_json::to_string(&project.team_ids).unwrap_or_default();
        stmt.execute(rusqlite::params![project.id, project.name, team_ids_json])?;
    }

    Ok(())
}

pub fn load_cached_projects(conn: &Connection) -> Result<Vec<Project>, AppError> {
    let mut stmt = conn.prepare("SELECT id, name, team_ids FROM projects ORDER BY name")?;
    let projects = stmt.query_map([], |row| {
        let team_ids_str: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
        let team_ids: Vec<String> = serde_json::from_str(&team_ids_str).unwrap_or_default();
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            team_ids,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(projects)
}

pub fn upsert_labels(conn: &Connection, labels: &[Label]) -> Result<(), AppError> {
    let mut stmt = conn.prepare(
        "INSERT INTO labels (id, name, color, team_id, cached_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET name = ?2, color = ?3, team_id = ?4, cached_at = datetime('now')"
    )?;

    for label in labels {
        stmt.execute(rusqlite::params![label.id, label.name, label.color, label.team_id])?;
    }

    Ok(())
}

pub fn load_cached_labels(conn: &Connection) -> Result<Vec<Label>, AppError> {
    let mut stmt = conn.prepare("SELECT id, name, color, team_id FROM labels ORDER BY name")?;
    let labels = stmt.query_map([], |row| {
        Ok(Label {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            team_id: row.get(3)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(labels)
}

pub fn upsert_workflow_states(conn: &Connection, states: &[IssueStatus]) -> Result<(), AppError> {
    let mut stmt = conn.prepare(
        "INSERT INTO workflow_states (id, name, team_id, color, position, cached_at)
         VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
            name = ?2, team_id = ?3, color = ?4, position = ?5, cached_at = datetime('now')"
    )?;

    for state in states {
        stmt.execute(rusqlite::params![
            state.id,
            state.name,
            state.team_id,
            state.color,
            state.position,
        ])?;
    }

    Ok(())
}

pub fn load_workflow_states(conn: &Connection) -> Result<Vec<IssueStatus>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, team_id, color, position FROM workflow_states ORDER BY team_id, position"
    )?;
    let states = stmt.query_map([], |row| {
        Ok(IssueStatus {
            id: row.get(0)?,
            name: row.get(1)?,
            team_id: row.get(2)?,
            color: row.get(3)?,
            position: row.get(4)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(states)
}
