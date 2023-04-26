use std::sync::Arc;

use super::*;

trait TestTrait: Send + Sync {
    fn cheers(&self);
}

#[derive(Default)]
struct SecretImpl {}
impl TestTrait for SecretImpl {
    fn cheers(&self) {
        println!("here is the secret ingredient");
    }
}

trait TestActionable {
    fn get_helper(&self) -> Arc<dyn TestTrait>;
}

struct ConcreteActionable {
    helper: Arc<dyn TestTrait>,
}

impl ConcreteActionable {
    fn new(helper: Arc<dyn TestTrait>) -> Self {
        Self { helper }
    }
}

impl TestActionable for ConcreteActionable {
    fn get_helper(&self) -> Arc<dyn TestTrait> {
        self.helper.clone()
    }
}

struct CyclicalA;
impl CyclicalA {
    fn with(_: Arc<CyclicalB>) -> Self {
        Self
    }
}
struct CyclicalB;
impl CyclicalB {
    fn with(_: Arc<CyclicalA>) -> Self {
        Self
    }
}

struct SimpleAction;

impl SimpleAction {
    fn create() -> Self {
        SimpleAction
    }

    fn callme(&self) {
        println!("The simple action was called");
    }
}

#[derive(Default)]
struct TestModule {}

resolve_singleton!(TestModule, dyn TestTrait: SecretImpl);

// define cyclical resolution rules
resolve_singleton!(TestModule, CyclicalA, with, Arc<CyclicalB>);
resolve_singleton!(TestModule, CyclicalB, with, Arc<CyclicalA>);

resolve_instance!(TestModule, SimpleAction, SimpleActionFactory, create);

resolve_instance!(
    TestModule,
    dyn TestActionable: ConcreteActionable,
    ActionableFactory,
    new,
    arg1: Arc<dyn TestTrait>
);

// Disable clippy lint on the comparison of fat pointers:
// this is only test code, the issue should not arise in this context
// and should be properly fixed in future rust versions
// * https://github.com/rust-lang/rust/pull/80505
// * https://stackoverflow.com/questions/67109860/how-to-compare-trait-objects-within-an-arc
#[allow(clippy::vtable_address_comparisons)]
#[test]
fn resolve_singleton() -> Result<(), WiringError> {
    let resolver = Hanami::new(TestModule {});

    let v1: Arc<dyn TestTrait> = resolver.inject()?;
    let v2: Arc<dyn TestTrait> = resolver.inject()?;

    v1.cheers();
    assert!(Arc::ptr_eq(&v1, &v2));

    // retrieve two on-demand instances: they are different but share the same helper
    let a1: Box<dyn TestActionable> = resolver.inject()?;
    let a2: Box<dyn TestActionable> = resolver.inject()?;
    let (h1, h2) = (a1.get_helper(), a2.get_helper());
    assert!(Arc::ptr_eq(&h1, &h2));

    let simple_action: SimpleAction = resolver.inject()?;
    simple_action.callme();

    Ok(())
}

#[allow(clippy::vtable_address_comparisons)]
#[test]
fn set_provider_early() -> Result<(), WiringError> {
    let mut resolver = Hanami::new(TestModule {});

    let singleton: Arc<dyn TestTrait> = Arc::new(SecretImpl::default());
    resolver.set_provider(SingletonProvider::build(singleton.clone()))?;

    let v1: Arc<dyn TestTrait> = resolver.inject()?;
    assert!(Arc::ptr_eq(&v1, &singleton));

    Ok(())
}

#[allow(clippy::vtable_address_comparisons)]
#[test]
fn set_provider_late() -> Result<(), WiringError> {
    let mut resolver = Hanami::new(TestModule {});

    let v1: Arc<dyn TestTrait> = resolver.inject()?;

    let singleton: Arc<dyn TestTrait> = Arc::new(SecretImpl::default());
    assert!(resolver
        .set_provider(SingletonProvider::build(singleton.clone()))
        .is_err());

    let v2: Arc<dyn TestTrait> = resolver.inject()?;
    assert!(!Arc::ptr_eq(&v1, &singleton));
    assert!(Arc::ptr_eq(&v1, &v2));

    Ok(())
}

#[test]
fn detect_cyclical() -> Result<(), WiringError> {
    let resolver = Hanami::new(TestModule {});
    let v1: Result<Arc<CyclicalA>, WiringError> = resolver.inject();
    matches!(v1, Err(WiringError::AlreadyResolved));
    Ok(())
}
