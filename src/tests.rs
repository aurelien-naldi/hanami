use std::sync::Arc;

use crate::{resolve, resolve_instance, resolve_singleton};

use super::Hanami;

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

struct TestModule;
struct TestModuleWrapper<T>(T);

resolve_singleton!(TestModule, dyn TestTrait => SecretImpl::default);

// define cyclical resolution rules
resolve_singleton!(TestModule,
    CyclicalA => CyclicalA::with,
    CyclicalB => CyclicalB::with
);

resolve_instance!(TestModule, SimpleAction => SimpleAction::create);

resolve_instance!(TestModule, Box: dyn TestActionable => ConcreteActionable : ConcreteActionable::new);

fn is_same_ptr<T: ?Sized>(a1: &Arc<T>, a2: &Arc<T>) -> bool {
    Arc::ptr_eq(a1, a2)
}

#[test]
fn resolve_singleton() {
    let resolver = Hanami::new(TestModule);

    let v1: Arc<dyn TestTrait> = resolver.inject();
    let v2: Arc<dyn TestTrait> = resolver.inject();

    v1.cheers();
    assert!(is_same_ptr(&v1, &v2));

    // retrieve two on-demand instances: they are different but share the same helper
    let a1: Box<dyn TestActionable> = resolver.inject();
    let a2: Box<dyn TestActionable> = resolver.inject();
    let (h1, h2) = (a1.get_helper(), a2.get_helper());
    assert!(is_same_ptr(&h1, &h2));

    let simple_action: SimpleAction = resolver.inject();
    simple_action.callme();
}

#[test]
fn set_provider_early() -> Result<(), resolve::WiringError> {
    let mut resolver = Hanami::new(TestModule);

    let singleton: Arc<dyn TestTrait> = Arc::new(SecretImpl::default());
    resolver.set_provider(resolve::SingletonProvider::build(singleton.clone()))?;

    let v1: Arc<dyn TestTrait> = resolver.inject();
    assert!(is_same_ptr(&v1, &singleton));

    Ok(())
}

#[test]
fn set_provider_late() {
    let mut resolver = Hanami::new(TestModule);

    let v1: Arc<dyn TestTrait> = resolver.inject();

    let singleton: Arc<dyn TestTrait> = Arc::new(SecretImpl::default());
    assert!(resolver
        .set_provider(resolve::SingletonProvider::build(singleton.clone()))
        .is_err());

    let v2: Arc<dyn TestTrait> = resolver.inject();
    assert!(!is_same_ptr(&v1, &singleton));
    assert!(is_same_ptr(&v1, &v2));
}

#[test]
#[should_panic]
fn detect_cyclical() {
    let resolver = Hanami::new(TestModule);
    let _v1: Arc<CyclicalA> = resolver.inject();
}
