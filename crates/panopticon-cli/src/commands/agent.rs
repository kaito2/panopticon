use anyhow::{Result, bail};
use panopticon_reputation::ReputationEngine;
use panopticon_types::{Agent, Capability};

use crate::AgentAction;
use crate::state::AppState;

pub async fn handle(action: AgentAction, state: &AppState) -> Result<()> {
    match action {
        AgentAction::Register { name, capabilities } => {
            let mut agent = Agent::new(&name);

            if let Some(caps) = capabilities {
                for cap_name in caps.split(',').map(|s| s.trim()) {
                    if !cap_name.is_empty() {
                        agent.capabilities.capabilities.push(Capability {
                            name: cap_name.to_string(),
                            proficiency: 0.5,
                            certified: false,
                            last_verified: None,
                        });
                    }
                }
            }

            let id = agent.id;
            println!("Registered agent: {} ({})", name, id);
            print_agent(&agent);
            state.agents.insert(id, agent);
        }

        AgentAction::List => {
            if state.agents.is_empty() {
                println!("No agents.");
                return Ok(());
            }
            for entry in state.agents.iter() {
                let a = entry.value();
                let caps: Vec<&str> = a
                    .capabilities
                    .capabilities
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect();
                println!(
                    "  {} {:?}  {}  [{}]",
                    a.id,
                    a.trust_level,
                    a.name,
                    caps.join(", ")
                );
            }
        }

        AgentAction::Get { id } => match state.agents.get(&id) {
            Some(entry) => print_agent(entry.value()),
            None => bail!("Agent not found: {id}"),
        },

        AgentAction::Reputation { id } => {
            let composite = state
                .reputation_engine
                .get_composite_score(id)
                .unwrap_or(0.5);
            let trust = ReputationEngine::compute_trust_level(composite);

            println!("Agent {id}");
            println!("  Composite score: {composite:.3}");
            println!("  Trust level:     {trust:?}");

            if let Some(entry) = state.agents.get(&id) {
                let r = &entry.value().reputation;
                println!("  Completion:      {:.3}", r.completion);
                println!("  Quality:         {:.3}", r.quality);
                println!("  Reliability:     {:.3}", r.reliability);
                println!("  Safety:          {:.3}", r.safety);
                println!("  Behavioral:      {:.3}", r.behavioral);
            }
        }
    }
    Ok(())
}

fn print_agent(a: &Agent) {
    println!("  ID:          {}", a.id);
    println!("  Name:        {}", a.name);
    println!("  Trust:       {:?}", a.trust_level);
    println!("  Available:   {}", a.available);
    println!("  Reputation:  {:.3}", a.reputation.composite());
    let caps: Vec<String> = a
        .capabilities
        .capabilities
        .iter()
        .map(|c| format!("{}({:.2})", c.name, c.proficiency))
        .collect();
    if !caps.is_empty() {
        println!("  Capabilities: {}", caps.join(", "));
    }
}
