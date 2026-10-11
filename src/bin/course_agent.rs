use course_agent_failure_ledger::{
    agent_loop::{record_failure, AgentStage, CourseRun},
    infrai::InfraiClient,
};
use std::process::ExitCode;

fn parse_stage(value: &str) -> Option<AgentStage> {
    match value {
        "delivery" => Some(AgentStage::CourseDelivery),
        "deadline" => Some(AgentStage::LearnerDeadline),
        "report" => Some(AgentStage::EducatorReport),
        _ => None,
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 6 {
        eprintln!("usage: course_agent <run-id> <course-id> <learner-id> <deadline> <delivery|deadline|report>");
        return ExitCode::from(2);
    }
    let Some(stage) = parse_stage(&args[5]) else {
        eprintln!("stage must be delivery, deadline, or report");
        return ExitCode::from(2);
    };
    let run = CourseRun {
        run_id: args[1].clone(),
        course_id: args[2].clone(),
        learner_id: args[3].clone(),
        deadline: args[4].clone(),
    };
    let client = match InfraiClient::from_env() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("configuration: {error}");
            return ExitCode::FAILURE;
        }
    };

    match record_failure(
        &client,
        &run,
        stage,
        "agent step could not produce a valid course action",
    )
    .await
    {
        Ok(decision) => {
            println!(
                "captured run={} stage={} decision={decision:?}",
                run.run_id,
                stage.as_str()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("capture: {error}");
            ExitCode::FAILURE
        }
    }
}
