use std::process::Command;

pub struct BranchStatus {
    pub is_current: bool,
    pub ahead: u32,
    pub behind: u32,
    pub dirty: bool,
}

impl BranchStatus {
    pub fn display_string(&self, branch: &str) -> String {
        let mut parts = vec![branch.to_string()];
        if self.ahead > 0 {
            parts.push(format!("↑{}", self.ahead));
        }
        if self.behind > 0 {
            parts.push(format!("↓{}", self.behind));
        }
        if self.dirty {
            parts.push("*".to_string());
        }
        parts.join(" ")
    }
}

/// List local branch names in a git repo
pub fn list_branches(working_dir: &str) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["branch", "--format=%(refname:short)"])
        .current_dir(working_dir)
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let branches = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    Ok(branches)
}

/// Get status info for a specific branch
pub fn get_branch_status(working_dir: &str, branch: &str) -> Result<BranchStatus, String> {
    // Check current branch
    let current = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(working_dir)
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    let current_branch = String::from_utf8_lossy(&current.stdout).trim().to_string();
    let is_current = current_branch == branch;

    // Check ahead/behind vs upstream
    let (ahead, behind) = get_ahead_behind(working_dir, branch).unwrap_or((0, 0));

    // Check dirty state only if this is the current branch
    let dirty = if is_current {
        let status = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(working_dir)
            .output()
            .map_err(|e| format!("Failed to run git: {e}"))?;
        !String::from_utf8_lossy(&status.stdout).trim().is_empty()
    } else {
        false
    };

    Ok(BranchStatus {
        is_current,
        ahead,
        behind,
        dirty,
    })
}

fn get_ahead_behind(working_dir: &str, branch: &str) -> Result<(u32, u32), String> {
    let output = Command::new("git")
        .args(["rev-list", "--left-right", "--count", &format!("{branch}...{branch}@{{upstream}}")])
        .current_dir(working_dir)
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    if !output.status.success() {
        return Ok((0, 0)); // No upstream, that's fine
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = text.trim().split('\t').collect();
    if parts.len() == 2 {
        let ahead = parts[0].parse().unwrap_or(0);
        let behind = parts[1].parse().unwrap_or(0);
        Ok((ahead, behind))
    } else {
        Ok((0, 0))
    }
}

/// Create a new local branch (does not checkout)
pub fn create_branch(working_dir: &str, branch_name: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(["branch", branch_name])
        .current_dir(working_dir)
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    Ok(())
}
