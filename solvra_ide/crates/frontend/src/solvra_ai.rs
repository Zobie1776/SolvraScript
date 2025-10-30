use std::collections::VecDeque;
use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::time;

#[derive(Debug, Clone)]
pub struct AiSuggestion {
    pub title: String,
    pub body: String,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub file: String,
    pub line: u32,
    pub message: String,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub file: String,
    pub position: (u32, u32),
    pub prefix: String,
}

#[async_trait]
pub trait SolvraAiAdapter: Send + Sync {
    async fn analyze_syntax_tree(&self, source: &str) -> Result<String>;
    async fn explain_error(&self, context: ErrorContext) -> Result<String>;
    async fn complete_code(&self, context: CompletionContext) -> Result<AiSuggestion>;
    async fn summarize_ast(&self, source: &str) -> Result<String>;
}

#[derive(Debug, Default, Clone)]
pub struct MockSolvraAiAdapter;

#[async_trait]
impl SolvraAiAdapter for MockSolvraAiAdapter {
    async fn analyze_syntax_tree(&self, source: &str) -> Result<String> {
        Ok(format!(
            "Syntax tree analysis placeholder for {} characters",
            source.len()
        ))
    }

    async fn explain_error(&self, context: ErrorContext) -> Result<String> {
        Ok(format!(
            "{}:{} -> {}",
            context.file, context.line, context.message
        ))
    }

    async fn complete_code(&self, context: CompletionContext) -> Result<AiSuggestion> {
        Ok(AiSuggestion {
            title: "Predictive completion".to_string(),
            body: format!("{} // completion", context.prefix),
            confidence: 0.42,
        })
    }

    async fn summarize_ast(&self, source: &str) -> Result<String> {
        Ok(format!(
            "AST summary placeholder for {} characters",
            source.len()
        ))
    }
}

#[derive(Debug)]
pub enum SolvraAiCommand {
    Analyze(String),
    Explain(ErrorContext),
    Complete(CompletionContext),
    Summarize(String),
    Shutdown,
}

#[derive(Debug)]
pub struct SolvraAiService<A: SolvraAiAdapter = MockSolvraAiAdapter> {
    adapter: A,
    history: VecDeque<AiSuggestion>,
}

impl<A: SolvraAiAdapter + Default> Default for SolvraAiService<A> {
    fn default() -> Self {
        Self {
            adapter: A::default(),
            history: VecDeque::with_capacity(32),
        }
    }
}

impl<A: SolvraAiAdapter> SolvraAiService<A> {
    pub async fn analyze_syntax_tree(&mut self, source: &str) -> Result<String> {
        self.adapter.analyze_syntax_tree(source).await
    }

    pub async fn explain_error(&mut self, context: ErrorContext) -> Result<String> {
        self.adapter.explain_error(context).await
    }

    pub async fn complete_code(&mut self, context: CompletionContext) -> Result<AiSuggestion> {
        let suggestion = self.adapter.complete_code(context).await?;
        self.history.push_front(suggestion.clone());
        while self.history.len() > 50 {
            self.history.pop_back();
        }
        Ok(suggestion)
    }

    pub async fn summarize_ast(&mut self, source: &str) -> Result<String> {
        self.adapter.summarize_ast(source).await
    }

    pub fn history(&self) -> impl Iterator<Item = &AiSuggestion> {
        self.history.iter()
    }

    pub async fn debounce_completion(
        &mut self,
        context: CompletionContext,
    ) -> Result<AiSuggestion> {
        time::sleep(Duration::from_millis(120)).await;
        self.complete_code(context).await
    }

    pub fn supports_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| matches!(ext, "svs" | "svc" | "rs" | "py" | "js" | "ts"))
            .unwrap_or(false)
    }
}

impl<A> SolvraAiService<A>
where
    A: SolvraAiAdapter + Clone + Send + Sync + 'static,
{
    pub async fn interactive_cli(&mut self) {
        let (sender, mut receiver) = mpsc::channel::<SolvraAiCommand>(4);
        let adapter = self.adapter.clone();
        tokio::spawn(async move {
            while let Some(command) = receiver.recv().await {
                match command {
                    SolvraAiCommand::Analyze(source) => {
                        if let Ok(summary) = adapter.analyze_syntax_tree(&source).await {
                            println!("[analysis] {}", summary);
                        }
                    }
                    SolvraAiCommand::Explain(ctx) => {
                        if let Ok(message) = adapter.explain_error(ctx).await {
                            println!("[explain] {}", message);
                        }
                    }
                    SolvraAiCommand::Complete(ctx) => {
                        if let Ok(suggestion) = adapter.complete_code(ctx).await {
                            println!("[complete] {}", suggestion.body);
                        }
                    }
                    SolvraAiCommand::Summarize(source) => {
                        if let Ok(summary) = adapter.summarize_ast(&source).await {
                            println!("[summary] {}", summary);
                        }
                    }
                    SolvraAiCommand::Shutdown => break,
                }
            }
        });

        println!("SolvraAI interactive shell. Type 'exit' to quit.");
        let mut line = String::new();
        while let Ok(n) = std::io::stdin().read_line(&mut line) {
            if n == 0 {
                break;
            }
            let trimmed = line.trim();
            if trimmed == "exit" {
                let _ = sender.send(SolvraAiCommand::Shutdown).await;
                break;
            } else if trimmed.starts_with("explain ") {
                let message = trimmed.trim_start_matches("explain ").to_string();
                let ctx = ErrorContext {
                    file: "stdin".into(),
                    line: 0,
                    message,
                    source: String::new(),
                };
                let _ = sender.send(SolvraAiCommand::Explain(ctx)).await;
            } else if trimmed.starts_with("analyze ") {
                let source = trimmed.trim_start_matches("analyze ").to_string();
                let _ = sender.send(SolvraAiCommand::Analyze(source)).await;
            } else if trimmed.starts_with("complete ") {
                let prefix = trimmed.trim_start_matches("complete ").to_string();
                let context = CompletionContext {
                    file: "stdin".into(),
                    position: (0, 0),
                    prefix,
                };
                let _ = sender.send(SolvraAiCommand::Complete(context)).await;
            } else if trimmed.starts_with("summarize ") {
                let source = trimmed.trim_start_matches("summarize ").to_string();
                let _ = sender.send(SolvraAiCommand::Summarize(source)).await;
            }
            line.clear();
        }
    }
}
