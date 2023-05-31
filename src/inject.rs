use std::any::{Any, TypeId};
use std::collections::hash_map::{Entry, HashMap};
use std::sync::{Arc, Mutex};

use crate::*;

/// Application-level dependency injection
pub trait Inject<T> {
    /// Obtain an instance of the target type.
    ///
    /// Return an error if the type could not be resolved
    fn inject(&self) -> T;

    /// Override the provider for the target type.
    ///
    /// Return an error if the type has already been resolved
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
    fn inject(&self) -> T {
        self.tm.lock().unwrap().inject_with(&self.resolver)
    }
    fn set_provider(&mut self, provider: Provider<T>) -> Result<(), WiringError> {
        let mut tm = self.tm.lock().unwrap();
        if tm.get_provider::<T>().is_some() {
            return Err(WiringError::AlreadyResolved);
        }
        tm.set_if_vacant::<Provider<T>>(TypeMapEntry::Ready(Box::new(provider)));
        Ok(())
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
    fn resolve_with<T: 'static>(&mut self, resolver: &impl Resolve<T>) -> &Provider<T> {
        if self.get_provider::<T>().is_none() {
            self.set_if_vacant::<Provider<T>>(TypeMapEntry::Resolving);
            let p = resolver.build_provider(self);
            self.set_if_resolving::<Provider<T>>(TypeMapEntry::Ready(Box::new(p)));
        }
        self.get_provider().unwrap()
    }

    fn resolved_with<R, T: ResolvedBy<R> + 'static>(&mut self, resolver: &R) -> &Provider<T> {
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

/// Mark a derived type as resolvable by a given resolver
///
/// This trait is implemented for tuples of resolved types
pub trait Injectable<R>: Sized {
    fn inject(resolver: &R, injector: &mut impl ProviderMap) -> Self;
    fn provide(resolver: &R, injector: &mut impl ProviderMap) -> Provider<Self>;
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
            ($(_injector.injected_with::<R,$param>(_resolver),)*)
        }

        #[inline]
        fn provide(
            _resolver: &R,
            _injector: &mut impl ProviderMap,
        ) -> Provider<Self> {
            Arc::new(($(_injector.resolved_with::<R,$param>(_resolver).clone(),)*))
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
