use std::{rc::Rc, sync::Arc, time::SystemTime};

use hanami::*;

// Define regular traits and implementor structs

trait Logger: Send + Sync {
    fn log(&self, content: &str);
}

trait DateLogger: Send + Sync {
    fn log_date(&self);
}

#[derive(Default, Debug)]
struct SomeHelper;

#[derive(Default)]
struct LoggerImpl;

impl Logger for LoggerImpl {
    fn log(&self, content: &str) {
        println!("{}", content);
    }
}

struct DateLoggerImpl {
    logger: Arc<dyn Logger>,
    helper: Arc<SomeHelper>,
}

impl DateLoggerImpl {
    fn new(logger: Arc<dyn Logger>, helper: Arc<SomeHelper>) -> Self {
        Self { logger, helper }
    }
}

impl DateLogger for DateLoggerImpl {
    fn log_date(&self) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        self.logger.log(&format!(
            "{}s since epoch with {:?}",
            now.as_secs(),
            &self.helper
        ));
    }
}

struct MyCommand {
    _date_logger: Arc<dyn DateLogger>,
}

impl MyCommand {
    fn new(_date_logger: Arc<dyn DateLogger>) -> Self {
        Self { _date_logger }
    }

    fn call_me(&self) {
        println!("the command works");
    }
}

// Define a resolver module implementing Resolve for our injected trait objects

/// A resolver module
struct LogResolver;
struct MyResolver {
    helper: LogResolver,
}
struct MyResolverWrapper<T>(T);

// Resolve a singleton of an explicit type
resolve_singleton!(LogResolver,
    SomeHelper => SomeHelper::default,
    dyn Logger => LoggerImpl::default,
    dyn DateLogger => DateLoggerImpl::new
);

// Declare proxy resolution rules
resolve_proxy!(MyResolver, LogResolver => helper);

resolve_instance!(MyResolver, Rc: MyCommand => MyCommand : MyCommand::new);

#[allow(clippy::vtable_address_comparisons)]
fn main() -> Result<(), WiringError> {
    let injector = Hanami::new(MyResolver {
        helper: LogResolver {},
    });

    let b: Arc<dyn DateLogger> = injector.inject();

    b.log_date();

    let c: Rc<MyCommand> = injector.inject();
    c.call_me();

    Ok(())
}
