//! TODO: crate-level doc!!

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Mutex;

/// Register and retrieve singletons of Any types
#[derive(Default)]
pub struct TypeMap {
    singletons: HashMap<TypeId, Box<dyn Any>>,
}

impl TypeMap {
    #[allow(clippy::map_entry)]
    pub fn get_or_insert_with<T: Any, F: FnOnce(&mut TypeMap) -> T>(&mut self, builder: F) -> &T {
        // We separate the initial test and the insert action instead of using entry::or_insert
        // to be able to forward a &mut to self, required to support inter-dependencies
        let type_id = TypeId::of::<T>();
        if !self.singletons.contains_key(&type_id) {
            let new_instance = Box::new(builder(self));
            self.singletons.insert(type_id, new_instance);
        }

        // We can chain unwraps here as we have just inserted missing entries and the downcast cannot fail
        self.singletons
            .get(&type_id)
            .unwrap()
            .downcast_ref::<T>()
            .unwrap()
    }
}

pub trait FromTypemap: Clone + 'static {
    fn create_using_typemap(typemap: &mut TypeMap) -> Box<Self>;

    fn resolve(typemap: &mut TypeMap) -> &Self {
        typemap.get_or_insert_with(|map| Self::create_using_typemap(map))
    }

    fn resolve_sync(typemap: &Mutex<TypeMap>) -> Self {
        let mut guard = typemap.lock().unwrap();
        Self::resolve(&mut guard).clone()
    }
}

#[cfg(test)]
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

    impl FromTypemap for Arc<dyn TestTrait> {
        fn create_using_typemap(_typemap: &mut TypeMap) -> Box<Self> {
            Box::new(Arc::new(SecretImpl {}))
        }
    }

    impl FromTypemap for Arc<dyn OtherTrait> {
        fn create_using_typemap(typemap: &mut TypeMap) -> Box<Self> {
            Box::new(Arc::new(OtherSecretImpl {
                helper: Arc::<dyn TestTrait>::resolve(typemap).clone(),
            }))
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
        let typemap = Mutex::new(TypeMap::default());

        // The service instances can be created and are cached
        let cpt = Arc::<dyn TestTrait>::resolve_sync(&typemap);
        let cpt2 = Arc::<dyn TestTrait>::resolve_sync(&typemap);
        assert!(Arc::ptr_eq(&cpt, &cpt2));

        // If the implementation of a trait depends on another service,
        // an implementation of this other service is now be in the cache
        let cpt3 = Arc::<dyn OtherTrait>::resolve_sync(&typemap);
        assert!(Arc::ptr_eq(&cpt, cpt3.helper()));
    }
}
