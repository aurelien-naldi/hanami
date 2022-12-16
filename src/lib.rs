//! TODO: crate-level doc!!

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// Marker trait to ensure Send + Sync on injectable traits
pub trait Interface: Send + Sync {}
impl<I: Send + Sync + 'static> Interface for I {}

/// A factory can create a new instance of the selected struct or interface
pub trait Factory<F, I: 'static + ?Sized> {
    fn build_new(&self, cache: &mut AnyCache) -> Arc<I>;

    fn get_or_build(&self, cache: &mut AnyCache) -> Arc<I> {
        // return a clone of the cached instance if available
        if let Some(o) = cache.cached::<Arc<I>>() {
            return Arc::clone(o);
        }

        // Create a new instance, add it to the cache and return a clone
        let new_instance = self.build_new(cache);
        cache.add_cache(Box::new(new_instance.clone()));
        new_instance
    }
}

pub trait Provider<I: 'static + ?Sized> {
    fn get(&self) -> Arc<I>;
}

impl<F: Factory<F, I>, I: Interface + 'static + ?Sized> Provider<I> for Registry<F> {
    fn get(&self) -> Arc<I> {
        let mut guard = self.cache.lock().unwrap();
        self.factory.get_or_build(&mut guard)
    }
}

pub struct Registry<F> {
    factory: Box<F>,
    cache: Mutex<AnyCache>,
}

/// Hold a cache of some type
#[derive(Default)]
pub struct AnyCache {
    singletons: HashMap<TypeId, Box<dyn Any>>,
}

impl AnyCache {
    pub fn cached<T: Any>(&self) -> Option<&T> {
        //        let guard = self.singletons.lock().unwrap();
        //        let r = guard?.clone();
        self.singletons.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    pub fn add_cache(&mut self, value: Box<dyn Any>) {
        self.singletons.insert((*value).type_id(), value);
        //        let mut guard = self.singletons.lock().unwrap();
        //        guard.insert((*value).type_id(), value);
    }
}

impl<F> Registry<F> {
    pub fn new(factory: F) -> Self {
        Registry {
            factory: Box::new(factory),
            cache: Mutex::default(),
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

    impl Factory<MyModule, dyn TestTrait> for MyModule {
        fn build_new(&self, cache: &mut AnyCache) -> Arc<dyn TestTrait> {
            let _o: Arc<dyn OtherTrait> = self.get_or_build(cache);
            Arc::new(SecretImpl {})
        }
    }

    impl Factory<MyModule, dyn OtherTrait> for MyModule {
        fn build_new(&self, _cache: &mut AnyCache) -> Arc<dyn OtherTrait> {
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

        // If the implementation of a trait depends on another service,
        // an implementation of this other service is now be in the cache
        let guard = registry.cache.lock().unwrap();
        let o1: Arc<dyn OtherTrait> = guard.cached().cloned().unwrap();
        drop(guard);
        let o2: Arc<dyn OtherTrait> = registry.get();
        assert!(Arc::ptr_eq(&o1, &o2));
    }
}
