use std::{sync::Arc, time::SystemTime};

use hanami::*;

// Define regular traits and implementor structs

trait Logger: Send + Sync {
    fn log(&self, content: &str);
}

trait DateLogger: Send + Sync {
    fn log_date(&self);
}

#[derive(Default)]
struct LoggerImpl;

impl Logger for LoggerImpl {
    fn log(&self, content: &str) {
        println!("{}", content);
    }
}

struct DateLoggerImpl {
    logger: Arc<dyn Logger>,
}

impl DateLoggerImpl {
    fn new(logger: Arc<dyn Logger>) -> Self {
        Self { logger }
    }
}

impl DateLogger for DateLoggerImpl {
    fn log_date(&self) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        self.logger.log(&format!("{}s since epoch", now.as_secs()));
    }
}

// Define a resolver module implementing Resolve for our injected trait objects

/// A resolver module
struct LogResolver;
struct MyResolver {
    helper: LogResolver,
}

// Resolve the Logger trait
resolve_singleton!(dyn Logger, LoggerImpl, LogResolver);

// Resolve the DateLogger trait
resolve_singleton!(dyn DateLogger, DateLoggerImpl, LogResolver, Arc<dyn Logger>);

// Declare proxy resolution rules
resolve_proxy!(MyResolver, LogResolver, helper);

#[allow(clippy::vtable_address_comparisons)]
fn main() -> Result<(), WiringError> {
    let injector = Hanami::new(MyResolver {
        helper: LogResolver {},
    });

    let b: Arc<dyn DateLogger> = injector.inject()?;

    b.log_date();

    Ok(())
}
