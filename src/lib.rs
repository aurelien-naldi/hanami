//! TODO: crate-level doc!!

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use thiserror::Error;

/// Store singletons of [Any] type
pub trait TypeMap {
    /// Check if the typemap contains a singleton of the selected type
    fn contains<T: Any>(&self) -> bool;

    /// Retrieve a stored singleton if it exists
    fn get<T: Any>(&self) -> Option<&T>;

    /// Insert a new singleton, or replace an existing one.
    /// After this binding, self.get<T>() will return Some(ref)
    fn bind<T: Any>(&mut self, data: T);

    /// Retrieve an existing singleton or create and register a new one
    fn get_or_insert_with<T: Any, F: FnOnce(&mut Self) -> T>(&mut self, builder: F) -> &T {
        if !self.contains::<T>() {
            let data = builder(self);
            self.bind(data)
        }
        self.get().unwrap()
    }
}

/// A [TypeMap] with an associated type
pub struct Registry<A> {
    singletons: HashMap<TypeId, Box<dyn Any>>,
    data: A,
}

impl<A> TypeMap for Registry<A> {
    fn contains<T: Any>(&self) -> bool {
        self.singletons.contains_key(&TypeId::of::<T>())
    }

    fn get<T: Any>(&self) -> Option<&T> {
        self.singletons.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    fn bind<T: Any>(&mut self, data: T) {
        self.singletons.insert(TypeId::of::<T>(), Box::new(data));
    }
}

impl<A> Registry<A> {
    pub fn new(data: A) -> Self {
        Self {
            singletons: HashMap::new(),
            data,
        }
    }

    pub fn associated(&self) -> &A {
        &self.data
    }
}

/// Either a shared singleton (an Arc) or a standalone instance (a Box)
pub enum Autowired<T: 'static + ?Sized> {
    Single(Box<T>),
    Shared(Arc<T>),
}

impl<T: ?Sized> Deref for Autowired<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Single(data) => data.deref(),
            Self::Shared(data) => data.deref(),
        }
    }
}

/// Obtain an instance from the associated injector
pub trait Autowire<I: ?Sized>
where
    Self: Sized,
{
    fn autowire(injector: &I) -> Result<Self, WiringError>;
}

/// Inject an instance of the associated type
pub trait Inject<T> {
    fn inject(&self) -> Result<T, WiringError>;
}

impl<I: ?Sized, T: Autowire<I>> Inject<T> for I {
    fn inject(&self) -> Result<T, WiringError> {
        T::autowire(self)
    }
}

/// Errors triggered during the autowiring process
#[derive(Error, Debug)]
pub enum WiringError {
    #[error("A singleton is missing from the read-only store")]
    SingletonIsMissing,
}

#[cfg(test)]
// Disable clippy lint on the comparison of fat pointers:
// this is only test code, the issue should not arise in this context
// and should be properly fixed in future rust versions
// * https://github.com/rust-lang/rust/pull/80505
// * https://stackoverflow.com/questions/67109860/how-to-compare-trait-objects-within-an-arc
mod tests {

    use std::sync::Arc;

    use super::*;

    trait TestTrait {
        fn cheers(&self);
    }

    trait OtherTrait {
        fn stamp(&self);
        fn helper(&self) -> &Arc<dyn TestTrait>;
    }

    struct SecretImpl {}
    impl TestTrait for SecretImpl {
        fn cheers(&self) {
            println!("here is the secret ingredient");
        }
    }
    struct OtherSecretImpl {
        helper: Arc<dyn TestTrait>,
    }
    impl OtherTrait for OtherSecretImpl {
        fn stamp(&self) {
            println!("actual secret stamping impl");
        }
        fn helper(&self) -> &Arc<dyn TestTrait> {
            &self.helper
        }
    }

    struct TestModule {}

    impl Autowire<Registry<TestModule>> for Autowired<dyn TestTrait> {
        fn autowire(_injector: &Registry<TestModule>) -> Result<Self, WiringError> {
            Ok(Autowired::Shared(Arc::new(SecretImpl {})))
        }
    }

    #[allow(clippy::vtable_address_comparisons)]
    #[test]
    fn it_works() {
        // // Create an empty registry
        // let typemap = Mutex::new(TypeMap::default());

        // // The service instances can be created and are cached
        // let cpt = Arc::<dyn TestTrait>::resolve_sync(&typemap);
        // let cpt2 = Arc::<dyn TestTrait>::resolve_sync(&typemap);
        // assert!(Arc::ptr_eq(&cpt, &cpt2));

        // // If the implementation of a trait depends on another service,
        // // an implementation of this other service is now be in the cache
        // let cpt3 = Arc::<dyn OtherTrait>::resolve_sync(&typemap);
        // assert!(Arc::ptr_eq(&cpt, cpt3.helper()));
    }

    #[test]
    fn with_autowire_api() -> Result<(), WiringError> {
        let registry = Registry::new(TestModule {});

        let v1: Autowired<dyn TestTrait> = registry.inject()?;
        v1.cheers();

        // let v2: Autowired<dyn TestTrait> = registry.inject()?;
        // v2.cheers();

        // assert!(!Arc::ptr_eq(&v1, &v2));

        Ok(())
    }
}
