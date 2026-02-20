use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use uuid::Uuid;

use panopticon_reputation::ReputationEngine;
use panopticon_types::{Agent, Task, TaskCharacteristics, TaskEvent};

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/tasks", post(create_task).get(list_tasks))
        .route("/api/v1/tasks/{task_id}", get(get_task))
        .route("/api/v1/tasks/{task_id}/transition", post(transition_task))
        .route("/api/v1/agents", post(register_agent).get(list_agents))
        .route("/api/v1/agents/{agent_id}", get(get_agent))
        .route(
            "/api/v1/agents/{agent_id}/reputation",
            get(get_agent_reputation),
        )
        .route("/health", get(health))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

#[derive(serde::Deserialize)]
struct CreateTaskRequest {
    name: String,
    description: String,
    #[serde(default)]
    characteristics: Option<TaskCharacteristics>,
    #[serde(default)]
    required_capabilities: Vec<String>,
}

async fn create_task(
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> (StatusCode, Json<Task>) {
    let mut task = Task::new(req.name, req.description);
    if let Some(chars) = req.characteristics {
        task.characteristics = chars;
    }
    task.required_capabilities = req.required_capabilities;

    let task_clone = task.clone();
    state.tasks.insert(task.id, task);

    (StatusCode::CREATED, Json(task_clone))
}

async fn list_tasks(State(state): State<AppState>) -> Json<Vec<Task>> {
    let tasks: Vec<Task> = state.tasks.iter().map(|r| r.value().clone()).collect();
    Json(tasks)
}

async fn get_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<Task>, StatusCode> {
    state
        .tasks
        .get(&task_id)
        .map(|t| Json(t.clone()))
        .ok_or(StatusCode::NOT_FOUND)
}

#[derive(serde::Deserialize)]
struct TransitionRequest {
    event: TaskEvent,
}

async fn transition_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    Json(req): Json<TransitionRequest>,
) -> Result<Json<Task>, (StatusCode, String)> {
    let mut task = state
        .tasks
        .get_mut(&task_id)
        .ok_or((StatusCode::NOT_FOUND, "Task not found".to_string()))?;

    task.apply_event(req.event)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Json(task.clone()))
}

#[derive(serde::Deserialize)]
struct RegisterAgentRequest {
    name: String,
}

async fn register_agent(
    State(state): State<AppState>,
    Json(req): Json<RegisterAgentRequest>,
) -> (StatusCode, Json<Agent>) {
    let agent = Agent::new(req.name);
    let agent_clone = agent.clone();
    state.agents.insert(agent.id, agent);

    (StatusCode::CREATED, Json(agent_clone))
}

async fn list_agents(State(state): State<AppState>) -> Json<Vec<Agent>> {
    let agents: Vec<Agent> = state.agents.iter().map(|r| r.value().clone()).collect();
    Json(agents)
}

async fn get_agent(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<Agent>, StatusCode> {
    state
        .agents
        .get(&agent_id)
        .map(|a| Json(a.clone()))
        .ok_or(StatusCode::NOT_FOUND)
}

#[derive(serde::Serialize)]
struct ReputationResponse {
    agent_id: Uuid,
    composite_score: f64,
    trust_level: panopticon_types::TrustLevel,
}

async fn get_agent_reputation(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<ReputationResponse>, StatusCode> {
    let composite = state
        .reputation_engine
        .get_composite_score(agent_id)
        .unwrap_or(0.5);
    let trust = ReputationEngine::compute_trust_level(composite);

    Ok(Json(ReputationResponse {
        agent_id,
        composite_score: composite,
        trust_level: trust,
    }))
}
