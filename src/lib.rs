//! TODO: crate-level doc!!

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// Marker trait to ensure Send + Sync on injectable traits
pub trait Interface: Send + Sync {}
impl<I: Send + Sync + 'static> Interface for I {}

/// Hold a cache of some type
pub trait SingletonCache {
    fn cached<T: Any + Clone>(&self) -> Option<T>;
    fn add_cache(&self, value: Box<dyn Any>);
}

/// A provider can build an instance of the selected struct / interface
pub trait Factory<I: Interface + 'static + ?Sized>: SingletonCache {
    fn build_new(&self) -> Arc<I>;

    fn get(&self) -> Arc<I> {
        if let Some(o) = self.cached::<Arc<I>>() {
            return o;
        }

        let a = self.build_new();
        self.add_cache(Box::new(a.clone()));

        a
    }
}

pub struct Registry<F> {
    _factory: F,
    singletons: Mutex<HashMap<TypeId, Box<dyn Any>>>,
}

impl<F> SingletonCache for Registry<F> {
    fn cached<T: Any + Clone>(&self) -> Option<T> {
        let guard = self.singletons.lock().unwrap();
        let r = guard.get(&TypeId::of::<T>())?.downcast_ref::<T>()?.clone();
        Some(r)
    }

    fn add_cache(&self, value: Box<dyn Any>) {
        let mut guard = self.singletons.lock().unwrap();
        guard.insert((*value).type_id(), value);
    }
}

impl<F> Registry<F> {
    pub fn new(factory: F) -> Self {
        Registry {
            _factory: factory,
            singletons: Mutex::default(),
        }
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use super::*;

    trait TestTrait: Interface {
        fn cheers(&self);
    }

    trait OtherTrait: Interface {
        fn stamp(&self);
    }

    struct SecretImpl {}

    impl TestTrait for SecretImpl {
        fn cheers(&self) {
            println!("here is the secret ingredient");
        }
    }
    struct SecretImplv2 {}

    impl TestTrait for SecretImplv2 {
        fn cheers(&self) {
            println!("here is the NEWEST secret ingredient");
        }
    }

    struct OtherSecretImpl {}

    impl OtherTrait for OtherSecretImpl {
        fn stamp(&self) {
            println!("actual secret stamping impl");
        }
    }

    struct MyModule {}

    impl Factory<dyn TestTrait> for Registry<MyModule> {
        fn build_new(&self) -> Arc<dyn TestTrait> {
            let _o: Arc<dyn OtherTrait> = self.get();
            Arc::new(SecretImpl {})
        }
    }

    impl Factory<dyn OtherTrait> for Registry<MyModule> {
        fn build_new(&self) -> Arc<dyn OtherTrait> {
            Arc::new(OtherSecretImpl {})
        }
    }

    // Disable clippy lint on the comparison of fat pointers:
    // this is only test code, the issue should not arise in this context
    // and should be properly fixed in future rust versions
    // * https://github.com/rust-lang/rust/pull/80505
    // * https://stackoverflow.com/questions/67109860/how-to-compare-trait-objects-within-an-arc
    #[allow(clippy::vtable_address_comparisons)]
    #[test]
    fn it_works() {
        // Create an empty registry
        let registry = Registry::new(MyModule {});

        // The service instances can be created and are cached
        let cpt: Arc<dyn TestTrait> = registry.get();
        let cpt2: Arc<dyn TestTrait> = registry.get();
        assert!(Arc::ptr_eq(&cpt, &cpt2));

        // We can force the creation of a new instance
        let cpt3: Arc<dyn TestTrait> = registry.build_new();
        assert!(!Arc::ptr_eq(&cpt, &cpt3));

        // The new instance does not update the cache. Maybe it should
        let cpt4: Arc<dyn TestTrait> = registry.get();
        assert!(Arc::ptr_eq(&cpt, &cpt4));
        assert!(!Arc::ptr_eq(&cpt3, &cpt4));

        // If the implementation of a trait depends on another service,
        // an implementation of this other service is now be in the cache
        let o1: Arc<dyn OtherTrait> = registry.cached().unwrap();
        let o2: Arc<dyn OtherTrait> = registry.get();
        assert!(Arc::ptr_eq(&o1, &o2));
    }
}
