use serde::{Deserialize, Serialize};

use super::ODataLinks;

/// http://redfish.dmtf.org/schemas/v1/Task.v1_7_1.json#/definitions/Task
/// The Task schema contains information about a task that the Redfish task service schedules or executes.
/// Tasks represent operations that take more time than a client typically wants to wait.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Task {
    #[serde(flatten)]
    pub odata: ODataLinks,
    pub id: String,
    #[serde(default)]
    pub messages: Vec<Message>,
    pub name: Option<String>,
    pub task_state: Option<TaskState>,
    pub task_status: Option<String>,
    pub task_monitor: Option<String>,
    pub percent_complete: Option<u32>,
}

/// https://redfish.dmtf.org/schemas/v1/Message.v1_1_2.json
/// The message that the Redfish service returns.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Message {
    pub message: String,
    #[serde(default)]
    pub message_args: Vec<String>,
    pub message_id: String,
    pub resolution: Option<String>,
    pub severity: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TaskState {
    New,
    Starting,
    Running,
    Suspended,
    Interrupted,
    Pending,
    Stopping,
    Completed,
    Killed,
    Exception,
    Service,
    Cancelling,
    Cancelled,
}
