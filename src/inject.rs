use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Mutex;

use crate::*;

/// Application-level dependency injection
pub trait Inject<T> {
    /// Obtain an instance of a given type.
    ///
    /// Return an error if the type has not been resolved at startup
    fn inject(&self) -> Result<T, WiringError>;
}

/// Dependency injection registry.
///
/// The [Resolve] implementations of the associated resolver are used
///  to derive [Inject] implementations on the registry itself.
pub struct Hanami<R> {
    tm: Mutex<TypeMap>,
    resolver: R,
}

impl<R> Hanami<R> {
    pub fn new(resolver: R) -> Self {
        Self {
            tm: Mutex::default(),
            resolver,
        }
    }

    pub fn get_resolver(&self) -> &R {
        &self.resolver
    }
}

/// Provide an Inject impl for all types resolved by Hanami's associated module
impl<T: 'static, M: Resolve<T>> Inject<T> for Hanami<M> {
    fn inject(&self) -> Result<T, WiringError> {
        self.tm.lock().unwrap().inject_with(&self.resolver)
    }
}

enum TypeMapEntry {
    Resolving,
    Ready(Box<dyn Any>),
}

enum TypeMapContent<'a, T> {
    None,
    Resolving,
    Mismatch,
    Ready(&'a T),
}

/// Store singletons of [Any] type
#[derive(Default)]
struct TypeMap(HashMap<TypeId, TypeMapEntry>);
impl TypeMap {
    /// Retrieve a stored singleton if it exists
    fn get<T: Any>(&self) -> TypeMapContent<T> {
        match self.0.get(&TypeId::of::<T>()) {
            None => TypeMapContent::None,
            Some(TypeMapEntry::Resolving) => TypeMapContent::Resolving,
            Some(TypeMapEntry::Ready(b)) => match b.downcast_ref::<T>() {
                None => TypeMapContent::Mismatch,
                Some(b) => TypeMapContent::Ready(b),
            },
        }
    }

    fn get_provider<T: 'static>(&self) -> Option<&Provider<T>> {
        match self.get::<Provider<T>>() {
            TypeMapContent::Ready(v) => Some(v),
            _ => None,
        }
    }

    /// Start resolving a type.
    /// After this binding, ```self.get<T>()``` will return ```Resolving```
    fn book<T: Any>(&mut self) {
        self.0.insert(TypeId::of::<T>(), TypeMapEntry::Resolving);
    }

    /// Insert a new singleton.
    /// After this binding, ```self.get<T>()``` will return Some(ref)
    fn bind<T: Any>(&mut self, data: T) {
        self.0
            .insert(TypeId::of::<T>(), TypeMapEntry::Ready(Box::new(data)));
    }
}

impl ProviderMap for TypeMap {
    fn resolve_with<T: 'static>(
        &mut self,
        resolver: &impl Resolve<T>,
    ) -> Result<&Provider<T>, WiringError> {
        if self.get_provider::<T>().is_none() {
            self.book::<T>();
            let p = resolver.build_provider(self)?;
            self.bind(p);
        }
        Ok(self.get_provider().unwrap())
    }
}
