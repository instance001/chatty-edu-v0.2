use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::settings::Settings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeworkAssignment {
    pub id: String,
    pub title: String,
    pub subject: String,
    pub year_level: String,
    pub due_at: Option<String>,
    pub instructions_md: String,
    #[serde(default)]
    pub attachments: Vec<String>,
    #[serde(default = "default_allow_games")]
    pub allow_games: bool,
    #[serde(default)]
    pub allow_ai_premark: bool,
    pub max_score: Option<i32>,
}

fn default_allow_games() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeworkPack {
    pub version: String,
    pub school_id: String,
    pub class_id: String,
    pub created_at: String,
    pub assignments: Vec<HomeworkAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerEntry {
    pub question: String,
    pub response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPremark {
    pub score: Option<i32>,
    pub feedback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeworkSubmission {
    pub version: String,
    pub school_id: String,
    pub class_id: String,
    pub assignment_id: String,
    pub student_id: String,
    pub student_name: String,
    pub submitted_at: String,
    #[serde(default)]
    pub answers_text: Option<String>,
    #[serde(default)]
    pub answers: Vec<AnswerEntry>,
    #[serde(default)]
    pub ai_premark: Option<AiPremark>,
    #[serde(default)]
    pub attachments: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SubmissionSummary {
    pub assignment_id: String,
    pub student_name: String,
    pub student_id: String,
    #[allow(dead_code)]
    pub submitted_at: String,
    pub score: Option<i32>,
    pub ai_score: Option<i32>,
    pub ai_feedback: Option<String>,
}

pub fn export_pack_template(base: &Path, school_id: &str, class_id: &str) -> io::Result<PathBuf> {
    let pack = HomeworkPack {
        version: "1.0".to_string(),
        school_id: school_id.to_string(),
        class_id: class_id.to_string(),
        created_at: iso_now(),
        assignments: vec![HomeworkAssignment {
            id: "hw-sample-001".to_string(),
            title: "Sample homework".to_string(),
            subject: "General".to_string(),
            year_level: "7".to_string(),
            due_at: None,
            instructions_md: "Add your instructions here.\n- Question 1\n- Question 2".to_string(),
            attachments: vec![],
            allow_games: false,
            allow_ai_premark: true,
            max_score: Some(100),
        }],
    };

    let dir = base.join("homework").join("assigned");
    fs::create_dir_all(&dir)?;
    let path = dir.join("homework_pack_template.json");
    let json = serde_json::to_string_pretty(&pack)?;
    fs::write(&path, json)?;
    Ok(path)
}

pub fn create_pack(
    base: &Path,
    school_id: &str,
    class_id: &str,
    assignment: HomeworkAssignment,
) -> io::Result<PathBuf> {
    create_pack_multi(base, school_id, class_id, vec![assignment])
}

pub fn create_pack_multi(
    base: &Path,
    school_id: &str,
    class_id: &str,
    assignments: Vec<HomeworkAssignment>,
) -> io::Result<PathBuf> {
    let pack = HomeworkPack {
        version: "1.0".to_string(),
        school_id: school_id.to_string(),
        class_id: class_id.to_string(),
        created_at: iso_now(),
        assignments,
    };

    let dir = base.join("homework").join("assigned");
    fs::create_dir_all(&dir)?;
    let filename = format!(
        "homework_pack_{}_{}.json",
        class_id,
        pack.created_at.replace(':', "-")
    );
    let path = dir.join(filename);
    let json = serde_json::to_string_pretty(&pack)?;
    fs::write(&path, json)?;
    Ok(path)
}

pub fn load_pack_from_file(path: &Path) -> io::Result<HomeworkPack> {
    let contents = fs::read_to_string(path)?;
    let pack: HomeworkPack = serde_json::from_str(&contents)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("pack parse error: {e}")))?;
    Ok(pack)
}

pub fn find_latest_pack(base: &Path) -> io::Result<Option<(PathBuf, HomeworkPack)>> {
    let dir = base.join("homework").join("assigned");
    if !dir.exists() {
        return Ok(None);
    }

    let mut newest: Option<(PathBuf, SystemTime)> = None;
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().map(|e| e == "json").unwrap_or(false)
            && path
                .file_name()
                .map(|n| n.to_string_lossy().contains("homework_pack"))
                .unwrap_or(false)
        {
            let meta = entry.metadata()?;
            if let Ok(modified) = meta.modified() {
                match &newest {
                    Some((_, ts)) if *ts >= modified => {}
                    _ => newest = Some((path, modified)),
                }
            }
        }
    }

    if let Some((path, _)) = newest {
        let contents = fs::read_to_string(&path)?;
        let pack: HomeworkPack = serde_json::from_str(&contents)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("pack parse error: {e}")))?;
        Ok(Some((path, pack)))
    } else {
        Ok(None)
    }
}

pub fn apply_pack_policy(settings: &mut Settings, pack: &HomeworkPack) {
    let any_games_disallowed = pack.assignments.iter().any(|a| !a.allow_games);
    if any_games_disallowed {
        settings.game.enabled = false;
        settings.game.games_in_class_allowed = false;
    }
}

pub fn save_submission_with_answers(
    base: &Path,
    settings: &Settings,
    assignment_id: &str,
    answers_text: &str,
    attachments: &[String],
) -> io::Result<PathBuf> {
    let dir = base.join("homework").join("completed");
    fs::create_dir_all(&dir)?;

    let student_id = if settings.student.student_id.is_empty() {
        "student-id".to_string()
    } else {
        settings.student.student_id.clone()
    };
    let student_name = if settings.student.student_name.is_empty() {
        "Student".to_string()
    } else {
        settings.student.student_name.clone()
    };
    let class_id = if settings.student.class_id.is_empty() {
        "class".to_string()
    } else {
        settings.student.class_id.clone()
    };

    let premark = simple_premark(answers_text);

    let submission = HomeworkSubmission {
        version: "1.0".to_string(),
        school_id: "school".to_string(),
        class_id,
        assignment_id: assignment_id.to_string(),
        student_id: student_id.clone(),
        student_name: student_name.clone(),
        submitted_at: iso_now(),
        answers_text: Some(answers_text.to_string()),
        answers: vec![],
        ai_premark: Some(premark),
        attachments: attachments.to_vec(),
    };

    let filename = format!("submission_{}_{}.json", assignment_id, student_id);
    let path = dir.join(filename);
    let json = serde_json::to_string_pretty(&submission)?;
    fs::write(&path, json)?;
    Ok(path)
}

impl HomeworkSubmission {
    pub fn score_field(&self) -> Option<i32> {
        self.ai_premark.as_ref().and_then(|p| p.score)
    }
}

pub fn load_submission_summaries(base: &Path) -> io::Result<Vec<SubmissionSummary>> {
    let dir = base.join("homework").join("completed");
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }

    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || path.extension().map(|e| e != "json").unwrap_or(true) {
            continue;
        }
        let contents = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Ok(sub) = serde_json::from_str::<HomeworkSubmission>(&contents) {
            let ai_score = sub.ai_premark.as_ref().and_then(|p| p.score);
            let ai_feedback = sub.ai_premark.as_ref().and_then(|p| p.feedback.clone());
            out.push(SubmissionSummary {
                assignment_id: sub.assignment_id.clone(),
                student_name: sub.student_name.clone(),
                student_id: sub.student_id.clone(),
                submitted_at: sub.submitted_at.clone(),
                score: sub.score_field(),
                ai_score,
                ai_feedback,
            });
        }
    }
    Ok(out)
}

fn iso_now() -> String {
    let now = chrono::Utc::now();
    now.to_rfc3339()
}

fn simple_premark(text: &str) -> AiPremark {
    let len = text.trim().len();
    let score = if len > 400 {
        90
    } else if len > 200 {
        80
    } else if len > 100 {
        70
    } else if len > 40 {
        60
    } else {
        50
    };
    let feedback = if len < 50 {
        "Try adding more detail to your answers.".to_string()
    } else if len < 150 {
        "Good start—check if all parts are addressed.".to_string()
    } else {
        "Looks thorough. Review for accuracy and clarity.".to_string()
    };
    AiPremark {
        score: Some(score),
        feedback: Some(feedback),
    }
}
