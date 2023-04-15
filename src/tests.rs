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
#[derive(Default)]
struct TestModule {}

resolve_singleton!(dyn TestTrait, SecretImpl, TestModule);

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

    Ok(())
}
