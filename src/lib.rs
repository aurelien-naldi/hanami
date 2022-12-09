//! TODO: crate-level doc!!

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

/// Marker trait to ensure Send + Sync on injectable traits
pub trait Interface: Send + Sync {}
impl<I: Send + Sync + 'static> Interface for I {}


/// Hold a cache of some type
pub trait SingletonCache {
    fn cached<T: Any>(&self) -> Option<&T>;
    fn add_cache(&mut self, value: Box<dyn Any>);
}


/// A provider can build an instance of the selected struct / interface
pub trait Factory<I: Interface + 'static + ?Sized>: SingletonCache {
    fn build_new(&self) -> Arc<I>;

    fn get(&mut self) -> Arc<I> {

        if let Some(o) = self.cached::<Arc<I>>() {
            return o.clone();
        }

        let a = self.build_new();
        self.add_cache(Box::new(a.clone()));

        a
    }
}


pub struct Registry<F> {
    _factory: F,
    singletons: HashMap<TypeId, Box<dyn Any>>,
}

impl<F> SingletonCache for Registry<F> {
    fn cached<T: Any>(&self) -> Option<&T> {
        self.singletons
            .get(&TypeId::of::<T>())?
            .downcast_ref::<T>()
    }

    fn add_cache(&mut self, value: Box<dyn Any>) {
        self.singletons.insert((*value).type_id(), value);
    }
}

impl<F> Registry<F> {
    pub fn new(factory: F) -> Self {
        Registry {
            _factory: factory,
            singletons: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {

    use std::{ptr, sync::Arc};

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
            Arc::new(SecretImpl {})
        }
    }

    impl Factory<dyn OtherTrait> for Registry<MyModule> {
        fn build_new(&self) -> Arc<dyn OtherTrait> {
            Arc::new(OtherSecretImpl {})
        }
    }

    #[test]
    fn it_works() {
        let mut m = Registry::new(MyModule {});

        let d: Arc<dyn TestTrait> = m.get();

        d.cheers();

        let mut registry = Registry::new(MyModule {});

        let cpt: Arc<dyn TestTrait> = registry.get();
        let cpt2: Arc<dyn TestTrait> = registry.get();
        let cpt3: Arc<dyn TestTrait> = registry.build_new();
        
        assert!(ptr::eq(cpt.as_ref(), cpt2.as_ref()));
        assert!(!ptr::eq(cpt.as_ref(), cpt3.as_ref()));

        let _: Arc<dyn OtherTrait> = registry.get();
    }

 }
