//! Agent manager for spawning, tracking, and canceling agents.
//!
//! This module provides the infrastructure for managing multiple concurrent agents
//! that can handle various tasks in parallel.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::status::{AgentId, AgentState, AgentStatus};

/// Progress reporter that agents can use to update their progress.
#[derive(Clone)]
pub struct ProgressReporter {
    agent_id: AgentId,
    tx: mpsc::UnboundedSender<ProgressUpdate>,
}

/// Internal message for progress updates.
struct ProgressUpdate {
    agent_id: AgentId,
    progress: u8,
}

impl ProgressReporter {
    /// Reports progress (0-100) for this agent.
    pub fn report(&self, progress: u8) {
        let _ = self.tx.send(ProgressUpdate {
            agent_id: self.agent_id,
            progress: progress.min(100),
        });
    }

    /// Reports progress with a description update.
    pub fn report_with_description(&self, progress: u8, _description: &str) {
        // For now, just report progress. Description updates can be added later.
        self.report(progress);
    }
}

/// Agent manager handles spawning, tracking, and canceling multiple agents.
pub struct AgentManager {
    agents: Arc<Mutex<HashMap<AgentId, ManagedAgent>>>,
    next_id: Arc<Mutex<u64>>,
    progress_tx: mpsc::UnboundedSender<ProgressUpdate>,
    progress_rx: Arc<Mutex<mpsc::UnboundedReceiver<ProgressUpdate>>>,
}

/// Internal representation of a managed agent.
#[allow(dead_code)]
struct ManagedAgent {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    description: String,
    status: AgentStatus,
    handle: JoinHandle<Result<String, String>>,
    cancel_tx: mpsc::Sender<()>,
}

impl AgentManager {
    /// Creates a new agent manager.
    pub fn new() -> Self {
        let (progress_tx, progress_rx) = mpsc::unbounded_channel();
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(0)),
            progress_tx,
            progress_rx: Arc::new(Mutex::new(progress_rx)),
        }
    }

    /// Spawns a new agent with the given name, description, and task.
    ///
    /// Returns the agent ID.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use coding_agent_cli::agents::AgentManager;
    ///
    /// # async fn example() {
    /// let manager = AgentManager::new();
    ///
    /// // Spawn an agent to perform a background task
    /// let agent_id = manager.spawn(
    ///     "file-processor".to_string(),
    ///     "Processing large file".to_string(),
    ///     || {
    ///         // Do some work
    ///         std::thread::sleep(std::time::Duration::from_secs(1));
    ///         Ok("Processed 1000 lines".to_string())
    ///     }
    /// );
    ///
    /// // Wait for completion
    /// let result = manager.wait(agent_id).await;
    /// println!("Agent result: {:?}", result);
    /// # }
    /// ```
    pub fn spawn<F>(&self, name: String, description: String, task: F) -> AgentId
    where
        F: FnOnce() -> Result<String, String> + Send + 'static,
    {
        self.spawn_with_progress(name, description, |_| task())
    }

    /// Spawns a new agent with progress reporting capabilities.
    ///
    /// The task function receives a `ProgressReporter` that it can use to
    /// report progress during execution.
    ///
    /// Returns the agent ID.
    pub fn spawn_with_progress<F>(&self, name: String, description: String, task: F) -> AgentId
    where
        F: FnOnce(ProgressReporter) -> Result<String, String> + Send + 'static,
    {
        // Generate unique ID
        let id = {
            let mut next = self.next_id.lock().unwrap();
            let id = *next;
            *next += 1;
            AgentId(id)
        };

        // Create cancellation channel
        let (cancel_tx, mut cancel_rx) = mpsc::channel(1);

        // Create progress reporter for this agent
        let reporter = ProgressReporter {
            agent_id: id,
            tx: self.progress_tx.clone(),
        };

        // Spawn the agent task
        let agents_clone = Arc::clone(&self.agents);
        let id_clone = id;

        let handle = tokio::spawn(async move {
            // Update status to running
            {
                let mut agents = agents_clone.lock().unwrap();
                if let Some(agent) = agents.get_mut(&id_clone) {
                    agent.status.state = AgentState::Running;
                }
            }

            // Run the task or check for cancellation
            tokio::select! {
                result = tokio::task::spawn_blocking(move || task(reporter)) => {
                    match result {
                        Ok(task_result) => task_result,
                        Err(e) => Err(format!("Task panic: {}", e)),
                    }
                }
                _ = cancel_rx.recv() => {
                    Err("Agent cancelled".to_string())
                }
            }
        });

        // Create and store the managed agent
        let managed_agent = ManagedAgent {
            name: name.clone(),
            description: description.clone(),
            status: AgentStatus {
                id,
                name,
                description,
                state: AgentState::Queued,
                progress: 0,
            },
            handle,
            cancel_tx,
        };

        self.agents.lock().unwrap().insert(id, managed_agent);

        id
    }

    /// Spawns a new async agent with progress reporting capabilities.
    ///
    /// Unlike `spawn_with_progress`, this method accepts an async function,
    /// allowing for true concurrent execution without blocking the thread pool.
    /// This is ideal for I/O-bound operations like network calls, file I/O, etc.
    ///
    /// Returns the agent ID.
    pub fn spawn_async<F, Fut>(&self, name: String, description: String, task: F) -> AgentId
    where
        F: FnOnce(ProgressReporter) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<String, String>> + Send + 'static,
    {
        // Generate unique ID
        let id = {
            let mut next = self.next_id.lock().unwrap();
            let id = *next;
            *next += 1;
            AgentId(id)
        };

        // Create cancellation channel
        let (cancel_tx, mut cancel_rx) = mpsc::channel(1);

        // Create progress reporter for this agent
        let reporter = ProgressReporter {
            agent_id: id,
            tx: self.progress_tx.clone(),
        };

        // Spawn the agent task
        let agents_clone = Arc::clone(&self.agents);
        let id_clone = id;

        let handle = tokio::spawn(async move {
            // Update status to running
            {
                let mut agents = agents_clone.lock().unwrap();
                if let Some(agent) = agents.get_mut(&id_clone) {
                    agent.status.state = AgentState::Running;
                }
            }

            // Run the async task or check for cancellation
            tokio::select! {
                result = task(reporter) => {
                    result
                }
                _ = cancel_rx.recv() => {
                    Err("Agent cancelled".to_string())
                }
            }
        });

        // Create and store the managed agent
        let managed_agent = ManagedAgent {
            name: name.clone(),
            description: description.clone(),
            status: AgentStatus {
                id,
                name,
                description,
                state: AgentState::Queued,
                progress: 0,
            },
            handle,
            cancel_tx,
        };

        self.agents.lock().unwrap().insert(id, managed_agent);

        id
    }

    /// Updates the progress of an agent.
    pub fn update_progress(&self, id: AgentId, progress: u8) -> Result<(), String> {
        let mut agents = self.agents.lock().unwrap();
        let agent = agents
            .get_mut(&id)
            .ok_or_else(|| format!("Agent {:?} not found", id))?;

        agent.status.progress = progress.min(100);
        Ok(())
    }

    /// Cancels an agent by its ID.
    pub async fn cancel(&self, id: AgentId) -> Result<(), String> {
        // Get the cancel sender outside the lock
        let cancel_tx = {
            let agents = self.agents.lock().unwrap();
            let agent = agents
                .get(&id)
                .ok_or_else(|| format!("Agent {:?} not found", id))?;
            agent.cancel_tx.clone()
        };

        // Send cancellation signal without holding the lock
        if cancel_tx.send(()).await.is_err() {
            return Err(format!("Failed to send cancel signal to agent {:?}", id));
        }

        // Update the state
        let mut agents = self.agents.lock().unwrap();
        if let Some(agent) = agents.get_mut(&id) {
            agent.status.state = AgentState::Cancelled;
        }

        Ok(())
    }

    /// Gets the status of an agent by ID.
    pub fn get_status(&self, id: AgentId) -> Option<AgentStatus> {
        let agents = self.agents.lock().unwrap();
        agents.get(&id).map(|agent| agent.status.clone())
    }

    /// Gets the status of all agents.
    pub fn get_all_statuses(&self) -> Vec<AgentStatus> {
        let agents = self.agents.lock().unwrap();
        agents.values().map(|agent| agent.status.clone()).collect()
    }

    /// Checks if an agent is complete (either succeeded or failed).
    pub fn is_complete(&self, id: AgentId) -> bool {
        let agents = self.agents.lock().unwrap();
        if let Some(agent) = agents.get(&id) {
            matches!(
                agent.status.state,
                AgentState::Complete | AgentState::Failed | AgentState::Cancelled
            )
        } else {
            false
        }
    }

    /// Waits for an agent to complete and returns its result.
    ///
    /// Removes the agent from the manager after completion.
    pub async fn wait(&self, id: AgentId) -> Result<String, String> {
        // Take the agent out of the map
        let mut agent = {
            let mut agents = self.agents.lock().unwrap();
            agents
                .remove(&id)
                .ok_or_else(|| format!("Agent {:?} not found", id))?
        };

        // Wait for completion
        let result = agent.handle.await;

        // Update final state based on result
        

        match result {
            Ok(Ok(value)) => {
                agent.status.state = AgentState::Complete;
                agent.status.progress = 100;
                Ok(value)
            }
            Ok(Err(error)) => {
                agent.status.state = AgentState::Failed;
                Err(error)
            }
            Err(join_error) => {
                agent.status.state = AgentState::Failed;
                Err(format!("Agent task panicked: {}", join_error))
            }
        }
    }

    /// Cancels all running agents.
    pub async fn cancel_all(&self) -> Result<(), String> {
        let agent_ids: Vec<AgentId> = {
            let agents = self.agents.lock().unwrap();
            agents.keys().copied().collect()
        };

        for id in agent_ids {
            // Ignore errors for individual agents
            let _ = self.cancel(id).await;
        }

        Ok(())
    }

    /// Returns the count of active (non-complete) agents.
    pub fn active_count(&self) -> usize {
        let agents = self.agents.lock().unwrap();
        agents
            .values()
            .filter(|agent| {
                !matches!(
                    agent.status.state,
                    AgentState::Complete | AgentState::Failed | AgentState::Cancelled
                )
            })
            .count()
    }

    /// Cleans up completed agents from the manager.
    pub fn cleanup_completed(&self) {
        let mut agents = self.agents.lock().unwrap();
        agents.retain(|_, agent| {
            !matches!(
                agent.status.state,
                AgentState::Complete | AgentState::Failed | AgentState::Cancelled
            )
        });
    }

    /// Processes any pending progress updates from agents.
    ///
    /// This should be called periodically to update agent progress.
    /// Returns the number of progress updates processed.
    pub fn process_progress_updates(&self) -> usize {
        let mut count = 0;
        let mut rx = self.progress_rx.lock().unwrap();

        // Process all pending updates
        while let Ok(update) = rx.try_recv() {
            if let Ok(()) = self.update_progress(update.agent_id, update.progress) {
                count += 1;
            }
        }

        count
    }

    /// Waits for multiple agents and collects their results.
    ///
    /// Returns a vector of results in the same order as the input agent IDs.
    /// If any agent fails, the entire operation returns an error with details
    /// about which agents failed.
    pub async fn wait_all(&self, ids: Vec<AgentId>) -> Result<Vec<String>, String> {
        let mut results = Vec::new();
        let mut errors = Vec::new();

        for id in ids {
            match self.wait(id).await {
                Ok(result) => results.push(result),
                Err(e) => errors.push((id, e)),
            }
        }

        if errors.is_empty() {
            Ok(results)
        } else {
            let error_messages: Vec<String> = errors
                .iter()
                .map(|(id, e)| format!("Agent {:?}: {}", id, e))
                .collect();
            Err(format!(
                "{} agent(s) failed:\n{}",
                errors.len(),
                error_messages.join("\n")
            ))
        }
    }

    /// Waits for multiple agents in parallel and collects their results.
    ///
    /// More efficient than `wait_all` as it waits for all agents concurrently.
    /// Returns a vector of results in the same order as the input agent IDs.
    pub async fn wait_all_parallel(&self, ids: Vec<AgentId>) -> Result<Vec<String>, String> {
        let futures: Vec<_> = ids.into_iter().map(|id| self.wait(id)).collect();

        let results = futures::future::join_all(futures).await;

        let mut successes = Vec::new();
        let mut errors = Vec::new();

        for (idx, result) in results.into_iter().enumerate() {
            match result {
                Ok(value) => successes.push(value),
                Err(e) => errors.push((idx, e)),
            }
        }

        if errors.is_empty() {
            Ok(successes)
        } else {
            let error_messages: Vec<String> = errors
                .iter()
                .map(|(idx, e)| format!("Agent #{}: {}", idx, e))
                .collect();
            Err(format!(
                "{} agent(s) failed:\n{}",
                errors.len(),
                error_messages.join("\n")
            ))
        }
    }

    /// Aggregates results from multiple agents using a custom combiner function.
    ///
    /// This is useful when you want to merge results in a specific way,
    /// such as concatenating strings, merging JSON, etc.
    pub async fn aggregate_results<F, R>(
        &self,
        ids: Vec<AgentId>,
        initial: R,
        combiner: F,
    ) -> Result<R, String>
    where
        F: Fn(R, String) -> R,
        R: Clone,
    {
        let results = self.wait_all(ids).await?;
        Ok(results.into_iter().fold(initial, combiner))
    }

    /// Waits for any agent to complete and returns its result along with the ID.
    ///
    /// This is useful for racing multiple agents and taking the first successful result.
    /// Returns the agent ID and result of the first agent to complete.
    pub async fn wait_any(&self, ids: Vec<AgentId>) -> Result<(AgentId, String), String> {
        if ids.is_empty() {
            return Err("No agents provided".to_string());
        }

        let futures: Vec<_> = ids
            .into_iter()
            .map(|id| {
                Box::pin(async move {
                    let result = self.wait(id).await;
                    (id, result)
                })
            })
            .collect();

        let (id, result) = futures::future::select_all(futures).await.0;

        match result {
            Ok(value) => Ok((id, value)),
            Err(e) => Err(format!("Agent {:?}: {}", id, e)),
        }
    }

    /// Waits for the first successful result from multiple agents.
    ///
    /// If all agents fail, returns the last error encountered.
    pub async fn wait_first_success(&self, ids: Vec<AgentId>) -> Result<String, String> {
        if ids.is_empty() {
            return Err("No agents provided".to_string());
        }

        let futures: Vec<_> = ids.into_iter().map(|id| self.wait(id)).collect();

        let results = futures::future::join_all(futures).await;

        // Return the first success
        if let Some(value) = results.iter().find_map(|r| r.as_ref().ok()) {
            return Ok(value.clone());
        }

        // All failed - return the last error
        if let Some(Err(e)) = results.last() {
            Err(e.clone())
        } else {
            Err("All agents failed".to_string())
        }
    }
}

impl Default for AgentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_agent_lifecycle() {
        let manager = AgentManager::new();

        // Spawn an agent
        let id = manager.spawn(
            "test-agent".to_string(),
            "Testing agent lifecycle".to_string(),
            || {
                std::thread::sleep(Duration::from_millis(50));
                Ok("success".to_string())
            },
        );

        // Initially should be queued
        let status = manager.get_status(id).unwrap();
        assert_eq!(status.name, "test-agent");
        assert!(matches!(
            status.state,
            AgentState::Queued | AgentState::Running
        ));

        // Wait for completion
        let result = manager.wait(id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn test_agent_cancellation() {
        let manager = AgentManager::new();

        // Spawn a long-running agent
        let id = manager.spawn(
            "long-agent".to_string(),
            "Long running task".to_string(),
            || {
                std::thread::sleep(Duration::from_secs(10));
                Ok("should not see this".to_string())
            },
        );

        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Cancel the agent
        let cancel_result = manager.cancel(id).await;
        assert!(cancel_result.is_ok());

        // Wait for it to finish (should be cancelled)
        let result = manager.wait(id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cancelled"));
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let manager = AgentManager::new();

        // Spawn multiple agents
        let id1 = manager.spawn("agent-1".to_string(), "First agent".to_string(), || {
            std::thread::sleep(Duration::from_millis(100));
            Ok("result-1".to_string())
        });

        let id2 = manager.spawn("agent-2".to_string(), "Second agent".to_string(), || {
            std::thread::sleep(Duration::from_millis(100));
            Ok("result-2".to_string())
        });

        let id3 = manager.spawn("agent-3".to_string(), "Third agent".to_string(), || {
            std::thread::sleep(Duration::from_millis(100));
            Ok("result-3".to_string())
        });

        // Check active count
        assert_eq!(manager.active_count(), 3);

        // Wait for all to complete
        let (r1, r2, r3) = tokio::join!(manager.wait(id1), manager.wait(id2), manager.wait(id3));

        assert_eq!(r1.unwrap(), "result-1");
        assert_eq!(r2.unwrap(), "result-2");
        assert_eq!(r3.unwrap(), "result-3");

        // All should be done now
        assert_eq!(manager.active_count(), 0);
    }

    #[tokio::test]
    async fn test_agent_failure_handling() {
        let manager = AgentManager::new();

        // Spawn an agent that fails
        let id = manager.spawn(
            "failing-agent".to_string(),
            "This will fail".to_string(),
            || Err("intentional error".to_string()),
        );

        // Wait for it to fail
        let result = manager.wait(id).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "intentional error");
    }

    #[tokio::test]
    async fn test_result_aggregation() {
        let manager = AgentManager::new();

        // Spawn agents
        let id1 = manager.spawn("a1".to_string(), "desc".to_string(), || Ok("1".to_string()));
        let id2 = manager.spawn("a2".to_string(), "desc".to_string(), || Ok("2".to_string()));
        let id3 = manager.spawn("a3".to_string(), "desc".to_string(), || Ok("3".to_string()));

        // Collect results
        let r1 = manager.wait(id1).await.unwrap();
        let r2 = manager.wait(id2).await.unwrap();
        let r3 = manager.wait(id3).await.unwrap();

        let combined = format!("{}{}{}", r1, r2, r3);
        assert_eq!(combined, "123");
    }

    #[tokio::test]
    async fn test_wait_all_success() {
        let manager = AgentManager::new();

        let id1 = manager.spawn("a1".to_string(), "desc".to_string(), || {
            Ok("result1".to_string())
        });
        let id2 = manager.spawn("a2".to_string(), "desc".to_string(), || {
            Ok("result2".to_string())
        });
        let id3 = manager.spawn("a3".to_string(), "desc".to_string(), || {
            Ok("result3".to_string())
        });

        let results = manager.wait_all(vec![id1, id2, id3]).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], "result1");
        assert_eq!(results[1], "result2");
        assert_eq!(results[2], "result3");
    }

    #[tokio::test]
    async fn test_wait_all_with_failure() {
        let manager = AgentManager::new();

        let id1 = manager.spawn("a1".to_string(), "desc".to_string(), || {
            Ok("result1".to_string())
        });
        let id2 = manager.spawn("a2".to_string(), "desc".to_string(), || {
            Err("error2".to_string())
        });
        let id3 = manager.spawn("a3".to_string(), "desc".to_string(), || {
            Ok("result3".to_string())
        });

        let results = manager.wait_all(vec![id1, id2, id3]).await;

        assert!(results.is_err());
        let err = results.unwrap_err();
        assert!(err.contains("1 agent(s) failed"));
        assert!(err.contains("error2"));
    }

    #[tokio::test]
    async fn test_wait_all_parallel_success() {
        let manager = AgentManager::new();

        let id1 = manager.spawn("a1".to_string(), "desc".to_string(), || {
            std::thread::sleep(Duration::from_millis(50));
            Ok("fast1".to_string())
        });
        let id2 = manager.spawn("a2".to_string(), "desc".to_string(), || {
            std::thread::sleep(Duration::from_millis(50));
            Ok("fast2".to_string())
        });
        let id3 = manager.spawn("a3".to_string(), "desc".to_string(), || {
            std::thread::sleep(Duration::from_millis(50));
            Ok("fast3".to_string())
        });

        let start = std::time::Instant::now();
        let results = manager.wait_all_parallel(vec![id1, id2, id3]).await;
        let elapsed = start.elapsed();

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 3);

        // Should take ~50ms (parallel), not ~150ms (sequential)
        // Use generous timeout for CI runners which can be slow
        assert!(
            elapsed < Duration::from_millis(500),
            "Parallel execution took {:?}, expected < 500ms",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_wait_all_parallel_with_failure() {
        let manager = AgentManager::new();

        let id1 = manager.spawn("a1".to_string(), "desc".to_string(), || {
            Ok("ok1".to_string())
        });
        let id2 = manager.spawn("a2".to_string(), "desc".to_string(), || {
            Err("fail2".to_string())
        });
        let id3 = manager.spawn("a3".to_string(), "desc".to_string(), || {
            Err("fail3".to_string())
        });

        let results = manager.wait_all_parallel(vec![id1, id2, id3]).await;

        assert!(results.is_err());
        let err = results.unwrap_err();
        assert!(err.contains("2 agent(s) failed"));
    }

    #[tokio::test]
    async fn test_aggregate_results() {
        let manager = AgentManager::new();

        let id1 = manager.spawn("a1".to_string(), "desc".to_string(), || {
            Ok("apple".to_string())
        });
        let id2 = manager.spawn("a2".to_string(), "desc".to_string(), || {
            Ok("banana".to_string())
        });
        let id3 = manager.spawn("a3".to_string(), "desc".to_string(), || {
            Ok("cherry".to_string())
        });

        let result = manager
            .aggregate_results(vec![id1, id2, id3], String::new(), |acc, s| {
                if acc.is_empty() {
                    s
                } else {
                    format!("{}, {}", acc, s)
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "apple, banana, cherry");
    }

    #[tokio::test]
    async fn test_aggregate_results_with_failure() {
        let manager = AgentManager::new();

        let id1 = manager.spawn(
            "a1".to_string(),
            "desc".to_string(),
            || Ok("ok".to_string()),
        );
        let id2 = manager.spawn("a2".to_string(), "desc".to_string(), || {
            Err("failed".to_string())
        });

        let result = manager
            .aggregate_results(vec![id1, id2], 0, |acc, s| {
                acc + s.parse::<i32>().unwrap_or(0)
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_wait_any() {
        let manager = AgentManager::new();

        let id1 = manager.spawn("slow".to_string(), "desc".to_string(), || {
            std::thread::sleep(Duration::from_millis(200));
            Ok("slow-result".to_string())
        });
        let id2 = manager.spawn("fast".to_string(), "desc".to_string(), || {
            std::thread::sleep(Duration::from_millis(10));
            Ok("fast-result".to_string())
        });

        let result = manager.wait_any(vec![id1, id2]).await;

        assert!(result.is_ok());
        let (winner_id, winner_result) = result.unwrap();

        // The fast one should win
        assert_eq!(winner_id, id2);
        assert_eq!(winner_result, "fast-result");
    }

    #[tokio::test]
    async fn test_wait_any_empty() {
        let manager = AgentManager::new();

        let result = manager.wait_any(vec![]).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No agents provided"));
    }

    #[tokio::test]
    async fn test_wait_first_success() {
        let manager = AgentManager::new();

        let id1 = manager.spawn("fail1".to_string(), "desc".to_string(), || {
            std::thread::sleep(Duration::from_millis(10));
            Err("error1".to_string())
        });
        let id2 = manager.spawn("success".to_string(), "desc".to_string(), || {
            std::thread::sleep(Duration::from_millis(20));
            Ok("good-result".to_string())
        });
        let id3 = manager.spawn("fail2".to_string(), "desc".to_string(), || {
            std::thread::sleep(Duration::from_millis(30));
            Err("error2".to_string())
        });

        let result = manager.wait_first_success(vec![id1, id2, id3]).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "good-result");
    }

    #[tokio::test]
    async fn test_wait_first_success_all_fail() {
        let manager = AgentManager::new();

        let id1 = manager.spawn("fail1".to_string(), "desc".to_string(), || {
            Err("error1".to_string())
        });
        let id2 = manager.spawn("fail2".to_string(), "desc".to_string(), || {
            Err("error2".to_string())
        });

        let result = manager.wait_first_success(vec![id1, id2]).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        // Should return the last error
        assert!(err.contains("error2"));
    }

    #[tokio::test]
    async fn test_wait_first_success_empty() {
        let manager = AgentManager::new();

        let result = manager.wait_first_success(vec![]).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No agents provided"));
    }

    #[tokio::test]
    async fn test_update_progress() {
        let manager = AgentManager::new();

        let id = manager.spawn(
            "progress-agent".to_string(),
            "Testing progress updates".to_string(),
            || {
                std::thread::sleep(Duration::from_millis(100));
                Ok("done".to_string())
            },
        );

        // Update progress externally
        manager.update_progress(id, 50).unwrap();
        let status = manager.get_status(id).unwrap();
        assert_eq!(status.progress, 50);

        // Progress should not exceed 100
        manager.update_progress(id, 150).unwrap();
        let status = manager.get_status(id).unwrap();
        assert_eq!(status.progress, 100);

        manager.wait(id).await.unwrap();
    }

    #[tokio::test]
    async fn test_progress_reporter() {
        let manager = AgentManager::new();

        let id = manager.spawn_with_progress(
            "reporter-agent".to_string(),
            "Testing progress reporter".to_string(),
            |reporter| {
                // Report progress at different stages
                reporter.report(25);
                std::thread::sleep(Duration::from_millis(10));
                reporter.report(50);
                std::thread::sleep(Duration::from_millis(10));
                reporter.report(75);
                std::thread::sleep(Duration::from_millis(10));
                reporter.report(100);
                Ok("done".to_string())
            },
        );

        // Give agent time to start and report some progress
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Process progress updates
        let updates_processed = manager.process_progress_updates();
        assert!(updates_processed > 0);

        // Check that progress was updated
        let status = manager.get_status(id).unwrap();
        assert!(status.progress > 0);

        manager.wait(id).await.unwrap();
    }

    #[tokio::test]
    async fn test_progress_clamped_to_100() {
        let manager = AgentManager::new();

        let id = manager.spawn_with_progress(
            "clamped-agent".to_string(),
            "Testing progress clamping".to_string(),
            |reporter| {
                // Try to report progress over 100
                reporter.report(150);
                Ok("done".to_string())
            },
        );

        tokio::time::sleep(Duration::from_millis(50)).await;
        manager.process_progress_updates();

        let status = manager.get_status(id).unwrap();
        assert_eq!(status.progress, 100);

        manager.wait(id).await.unwrap();
    }

    #[tokio::test]
    async fn test_multiple_agents_progress() {
        let manager = AgentManager::new();

        let id1 = manager.spawn_with_progress(
            "agent-1".to_string(),
            "First agent".to_string(),
            |reporter| {
                reporter.report(33);
                std::thread::sleep(Duration::from_millis(50));
                reporter.report(66);
                std::thread::sleep(Duration::from_millis(50));
                reporter.report(100);
                Ok("result-1".to_string())
            },
        );

        let id2 = manager.spawn_with_progress(
            "agent-2".to_string(),
            "Second agent".to_string(),
            |reporter| {
                reporter.report(50);
                std::thread::sleep(Duration::from_millis(100));
                reporter.report(100);
                Ok("result-2".to_string())
            },
        );

        // Process updates periodically
        for _ in 0..5 {
            tokio::time::sleep(Duration::from_millis(30)).await;
            manager.process_progress_updates();
        }

        // Both agents should have made progress
        let status1 = manager.get_status(id1).unwrap();
        let status2 = manager.get_status(id2).unwrap();
        assert!(status1.progress > 0);
        assert!(status2.progress > 0);

        manager.wait(id1).await.unwrap();
        manager.wait(id2).await.unwrap();
    }

    #[tokio::test]
    async fn test_cancel_all() {
        let manager = AgentManager::new();

        // Spawn multiple agents
        manager.spawn("a1".to_string(), "desc".to_string(), || {
            std::thread::sleep(Duration::from_secs(10));
            Ok("".to_string())
        });
        manager.spawn("a2".to_string(), "desc".to_string(), || {
            std::thread::sleep(Duration::from_secs(10));
            Ok("".to_string())
        });
        manager.spawn("a3".to_string(), "desc".to_string(), || {
            std::thread::sleep(Duration::from_secs(10));
            Ok("".to_string())
        });

        // Give agents time to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(manager.active_count(), 3);

        // Cancel all
        manager.cancel_all().await.unwrap();

        // Wait a bit for cancellations to process
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Active count should still be 3 until we wait for them
        // But all should be in cancelled state
        let statuses = manager.get_all_statuses();
        assert_eq!(statuses.len(), 3);
    }

    #[tokio::test]
    async fn test_cleanup_completed() {
        let manager = AgentManager::new();

        // Spawn and complete agents
        let id1 = manager.spawn("a1".to_string(), "desc".to_string(), || Ok("1".to_string()));
        let id2 = manager.spawn("a2".to_string(), "desc".to_string(), || Ok("2".to_string()));

        manager.wait(id1).await.unwrap();
        manager.wait(id2).await.unwrap();

        // Before cleanup
        assert_eq!(manager.get_all_statuses().len(), 0); // wait() removes them

        // Spawn another, complete it without waiting
        let _id3 = manager.spawn("a3".to_string(), "desc".to_string(), || Ok("3".to_string()));
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Cleanup should remove completed ones
        manager.cleanup_completed();
    }

    #[tokio::test]
    async fn test_async_agent_basic() {
        let manager = AgentManager::new();

        // Spawn an async agent
        let id = manager.spawn_async(
            "async-agent".to_string(),
            "Testing async execution".to_string(),
            |_reporter| async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok("async result".to_string())
            },
        );

        // Initially should be queued
        let status = manager.get_status(id).unwrap();
        assert_eq!(status.name, "async-agent");
        assert!(matches!(
            status.state,
            AgentState::Queued | AgentState::Running
        ));

        // Wait for completion
        let result = manager.wait(id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "async result");
    }

    #[tokio::test]
    async fn test_async_agent_with_progress() {
        let manager = AgentManager::new();

        // Spawn an async agent that reports progress
        let id = manager.spawn_async(
            "progress-async".to_string(),
            "Testing async progress".to_string(),
            |reporter| async move {
                reporter.report(25);
                tokio::time::sleep(Duration::from_millis(20)).await;
                reporter.report(50);
                tokio::time::sleep(Duration::from_millis(20)).await;
                reporter.report(75);
                tokio::time::sleep(Duration::from_millis(20)).await;
                reporter.report(100);
                Ok("done".to_string())
            },
        );

        // Give agent time to start and report progress
        tokio::time::sleep(Duration::from_millis(30)).await;

        // Process progress updates
        let updates = manager.process_progress_updates();
        assert!(updates > 0);

        // Check progress was updated
        let status = manager.get_status(id).unwrap();
        assert!(status.progress > 0);

        manager.wait(id).await.unwrap();
    }

    #[tokio::test]
    async fn test_async_parallel_execution() {
        let manager = AgentManager::new();

        // Spawn multiple async agents that run truly in parallel
        let id1 = manager.spawn_async(
            "async-1".to_string(),
            "First async agent".to_string(),
            |_| async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok("result-1".to_string())
            },
        );

        let id2 = manager.spawn_async(
            "async-2".to_string(),
            "Second async agent".to_string(),
            |_| async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok("result-2".to_string())
            },
        );

        let id3 = manager.spawn_async(
            "async-3".to_string(),
            "Third async agent".to_string(),
            |_| async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok("result-3".to_string())
            },
        );

        // Measure time - should be ~100ms (parallel), not ~300ms (sequential)
        let start = std::time::Instant::now();
        let results = manager.wait_all_parallel(vec![id1, id2, id3]).await;
        let elapsed = start.elapsed();

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], "result-1");
        assert_eq!(results[1], "result-2");
        assert_eq!(results[2], "result-3");

        // Verify parallel execution - should complete in roughly the time of one task
        assert!(
            elapsed < Duration::from_millis(200),
            "Expected parallel execution to take ~100ms, took {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_async_agent_cancellation() {
        let manager = AgentManager::new();

        // Spawn a long-running async agent
        let id = manager.spawn_async(
            "long-async".to_string(),
            "Long async task".to_string(),
            |_| async {
                tokio::time::sleep(Duration::from_secs(10)).await;
                Ok("should not see this".to_string())
            },
        );

        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Cancel the agent
        let cancel_result = manager.cancel(id).await;
        assert!(cancel_result.is_ok());

        // Wait for it to finish (should be cancelled)
        let result = manager.wait(id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cancelled"));
    }

    #[tokio::test]
    async fn test_async_agent_failure() {
        let manager = AgentManager::new();

        // Spawn an async agent that fails
        let id = manager.spawn_async(
            "failing-async".to_string(),
            "This will fail".to_string(),
            |_| async { Err("intentional async error".to_string()) },
        );

        // Wait for it to fail
        let result = manager.wait(id).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "intentional async error");
    }

    #[tokio::test]
    async fn test_mixed_sync_async_agents() {
        let manager = AgentManager::new();

        // Spawn both sync and async agents
        let sync_id = manager.spawn("sync-agent".to_string(), "Sync task".to_string(), || {
            std::thread::sleep(Duration::from_millis(50));
            Ok("sync-result".to_string())
        });

        let async_id = manager.spawn_async(
            "async-agent".to_string(),
            "Async task".to_string(),
            |_| async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok("async-result".to_string())
            },
        );

        // Wait for both
        let results = manager.wait_all_parallel(vec![sync_id, async_id]).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], "sync-result");
        assert_eq!(results[1], "async-result");
    }

    #[tokio::test]
    async fn test_many_async_agents_parallel() {
        let manager = AgentManager::new();

        // Spawn many async agents to test true parallelism
        let mut ids = Vec::new();
        for i in 0..20 {
            let id = manager.spawn_async(
                format!("agent-{}", i),
                format!("Task {}", i),
                move |reporter| async move {
                    reporter.report(50);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    reporter.report(100);
                    Ok(format!("result-{}", i))
                },
            );
            ids.push(id);
        }

        // Measure time - with true async parallelism, should be ~100ms
        let start = std::time::Instant::now();
        let results = manager.wait_all_parallel(ids).await;
        let elapsed = start.elapsed();

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 20);

        // Verify parallel execution - 20 agents at 100ms each should not take 2 seconds
        assert!(
            elapsed < Duration::from_millis(500),
            "Expected parallel execution, took {:?}",
            elapsed
        );
    }
}
