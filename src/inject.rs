use std::any::{Any, TypeId};
use std::collections::hash_map::{Entry, HashMap};
use std::sync::Mutex;

use crate::*;

/// Application-level dependency injection
pub trait Inject<T> {
    /// Obtain an instance of a given type.
    ///
    /// Return an error if the type has not been resolved at startup
    fn inject(&self) -> Result<T, WiringError>;

    fn set_provider(&mut self, provider: Provider<T>) -> Result<(), WiringError>;
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
    fn set_provider(&mut self, provider: Provider<T>) -> Result<(), WiringError> {
        self.tm
            .lock()
            .unwrap()
            .set_if_vacant::<Provider<T>>(TypeMapEntry::Ready(Box::new(provider)))
    }
}

#[derive(Debug)]
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

    /// Fill a free spot
    fn set_if_vacant<T: Any>(&mut self, data: TypeMapEntry) -> Result<(), WiringError> {
        let Entry::Vacant(v) = self.0.entry(TypeId::of::<T>()) else {
            // TODO: extra work to detect cyclical dependencies?
            return Err(WiringError::AlreadyResolved);
        };
        v.insert(data);
        Ok(())
    }

    /// Fill a resolving spot
    fn set_if_resolving<T: Any>(&mut self, data: TypeMapEntry) -> Result<(), WiringError> {
        let Entry::Occupied(mut o) = self.0.entry(TypeId::of::<T>()) else {
            return Err(WiringError::AlreadyResolved);
        };
        // Check the occupied status
        o.insert(data);
        Ok(())
    }
}

impl ProviderMap for TypeMap {
    fn resolve_with<T: 'static>(
        &mut self,
        resolver: &impl Resolve<T>,
    ) -> Result<&Provider<T>, WiringError> {
        if self.get_provider::<T>().is_none() {
            self.set_if_vacant::<Provider<T>>(TypeMapEntry::Resolving)?;
            let p = resolver.build_provider(self)?;
            self.set_if_resolving::<Provider<T>>(TypeMapEntry::Ready(Box::new(p)))?;
        }
        Ok(self.get_provider().unwrap())
    }
}
