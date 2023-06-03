use std::any::{Any, TypeId};
use std::collections::hash_map::{Entry, HashMap};
use std::sync::{Arc, Mutex};

use crate::resolve::*;

/// Dependency injection registry.
///
/// This struct combines a [ProviderMap] with a resolver module.
/// It can then resolve and inject all types resolved by the resolver module.
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

    /// Obtain an instance of the target type.
    ///
    /// Return an error if the type could not be resolved
    pub fn inject<T: 'static + ResolvedBy<R>>(&self) -> T {
        self.tm
            .lock()
            .unwrap()
            .resolve_with(&self.resolver)
            .provide()
    }

    /// Override the provider for the target type.
    ///
    /// Return an error if the type has already been resolved
    pub fn set_provider<T>(&mut self, provider: Provider<T>) -> Result<(), WiringError>
    where
        T: 'static + ResolvedBy<R>,
    {
        let mut tm = self.tm.lock().unwrap();
        if tm.get_provider::<T>().is_some() {
            return Err(WiringError::AlreadyResolved);
        }
        tm.set_if_vacant::<Provider<T>>(TypeMapEntry::Ready(Box::new(provider)));
        Ok(())
    }

    /// Call a function after injecting all its parameters
    pub fn inject_and_call<F, I, O>(&self, f: F) -> O
    where
        I: Injectable<R>,
        F: Callable<I, O>,
    {
        let mut tm = self.tm.lock().unwrap();
        tm.inject_and_call(&self.resolver, f)
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
    fn set_if_vacant<T: Any>(&mut self, data: TypeMapEntry) {
        let Entry::Vacant(v) = self.0.entry(TypeId::of::<T>()) else {
            // TODO: extra work to detect cyclical dependencies?
            panic!("Entry is not vacant");
        };
        v.insert(data);
    }

    /// Fill a resolving spot
    fn set_if_resolving<T: Any>(&mut self, data: TypeMapEntry) {
        let Entry::Occupied(mut o) = self.0.entry(TypeId::of::<T>()) else {
            panic!("Entry is not occupied");
        };
        // Check the occupied status
        o.insert(data);
    }
}

impl ProviderMap for TypeMap {
    fn resolve_with<R, T: ResolvedBy<R> + 'static>(&mut self, resolver: &R) -> &Provider<T> {
        if self.get_provider::<T>().is_none() {
            self.set_if_vacant::<Provider<T>>(TypeMapEntry::Resolving);
            let p = T::build_provider(resolver, self);
            self.set_if_resolving::<Provider<T>>(TypeMapEntry::Ready(Box::new(p)));
        }
        self.get_provider().unwrap()
    }
}

/*
 * The following is used to inject up to 10 parameters into any function
 * inspired by https://nickbryan.co.uk/software/using-a-type-map-for-dependency-injection-in-rust/
 */

/// A Callable has a ```call``` function with a single argument and a single return type.
///
/// This trait is implemented for all functions with up to 10 arguments, using a tuple to
/// wrap them all in a single type.
pub trait Callable<Args, Ret> {
    fn call(&self, args: Args) -> Ret;
}

macro_rules! callable_tuple ({ $($param:ident)* } => {
    impl<Func, Ret, $($param,)*> Callable<($($param,)*), Ret> for Func
    where
        Func: Fn($($param),*) -> Ret,
    {
        #[inline]
        #[allow(non_snake_case)]
        fn call(&self, ($($param,)*): ($($param,)*)) -> Ret {
            (self)($($param,)*)
        }
    }

    // Extract such tuples for a list of parameter types
    #[allow(clippy::unused_unit)]
    impl<R, $($param: ResolvedBy<R> + 'static,)*> Injectable<R> for ($($param,)*) {
        #[inline]
        fn inject(
            _resolver: &R,
            _injector: &mut impl ProviderMap,
        ) -> Self {
            ($(_injector.resolve_with::<R,$param>(_resolver).provide(),)*)
        }

        #[inline]
        fn provide(
            _resolver: &R,
            _injector: &mut impl ProviderMap,
        ) -> Provider<Self> {
            Arc::new(($(_injector.resolve_with::<R,$param>(_resolver).clone(),)*))
        }
}

    // A tuple of providers can provide a tuple of instances
    #[allow(non_snake_case)]
    #[allow(clippy::unused_unit)]
    impl<$($param,)*> Provide<($($param,)*)> for ($( Arc<dyn Provide<$param>>,)*) {
        fn provide(&self) -> ($($param,)*) {
            let ($($param,)*) = self;
            ($($param.provide(),)*)
        }
    }
});

callable_tuple! {}
callable_tuple! { A }
callable_tuple! { A B }
callable_tuple! { A B C }
callable_tuple! { A B C D }
callable_tuple! { A B C D E }
callable_tuple! { A B C D E F }
callable_tuple! { A B C D E F G }
callable_tuple! { A B C D E F G H }
callable_tuple! { A B C D E F G H I }
callable_tuple! { A B C D E F G H I J }
