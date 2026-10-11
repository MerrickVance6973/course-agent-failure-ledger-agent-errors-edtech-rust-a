use crate::infrai::{self, CaptureContext, CaptureException, InfraiClient, InfraiError};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStage {
    CourseDelivery,
    LearnerDeadline,
    EducatorReport,
}

impl AgentStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CourseDelivery => "course-delivery",
            Self::LearnerDeadline => "learner-deadline",
            Self::EducatorReport => "educator-report",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseRun {
    pub run_id: String,
    pub course_id: String,
    pub learner_id: String,
    pub deadline: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopDecision {
    HaltLearnerRun,
    ContinueNextCourse,
}

#[derive(Debug, Error)]
pub enum AgentLoopError {
    #[error("could not record the agent failure: {0}")]
    Tracking(#[from] InfraiError),
}

pub fn decision_for(stage: AgentStage) -> LoopDecision {
    match stage {
        AgentStage::CourseDelivery | AgentStage::LearnerDeadline => LoopDecision::HaltLearnerRun,
        AgentStage::EducatorReport => LoopDecision::ContinueNextCourse,
    }
}

pub async fn record_failure(
    client: &InfraiClient,
    run: &CourseRun,
    stage: AgentStage,
    cause: &str,
) -> Result<LoopDecision, AgentLoopError> {
    let stage_name = stage.as_str();
    let title = format!("edtech-agent/{stage_name} failed");
    let message = format!("run {}: {cause}", run.run_id);
    let idempotency_key = format!("{}:{}:{}", run.run_id, run.learner_id, stage_name);
    let exception = CaptureException {
        title: &title,
        message: &message,
        exception: cause,
        level: "error",
        fingerprint: ["edtech-agent", stage_name, &run.course_id],
        context: CaptureContext {
            course_id: &run.course_id,
            learner_id: &run.learner_id,
            deadline: &run.deadline,
            stage: stage_name,
        },
        service: "course-agent-loop",
        idempotency_key: &idempotency_key,
    };

    // Canonical call: infrai::errors::capture
    infrai::errors::capture(client, &exception).await?;
    Ok(decision_for(stage))
}

#[cfg(test)]
mod tests {
    use super::{decision_for, AgentStage, LoopDecision};

    #[test]
    fn learner_critical_failures_halt_but_reporting_failure_does_not() {
        assert_eq!(
            decision_for(AgentStage::CourseDelivery),
            LoopDecision::HaltLearnerRun
        );
        assert_eq!(
            decision_for(AgentStage::LearnerDeadline),
            LoopDecision::HaltLearnerRun
        );
        assert_eq!(
            decision_for(AgentStage::EducatorReport),
            LoopDecision::ContinueNextCourse
        );
    }
}
