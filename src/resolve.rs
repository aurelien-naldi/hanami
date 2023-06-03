//! Traits and structs supporting the resolution rules
//!
//! The injection mechanism combines a generic singleton map based on the [std::any::Any] trait for
//! runtime reflection with specialised traits to support each target types, enabling to ensure that
//! a type is injectable at compile-time.
//!
//!  To inject a target type, we first need to define a provider and a resolver for this type.
//!
//! * The [Provide] trait indicates that a struct can provide an instance of the target type.
//!   The provider can be either a singleton of the target type or a factory for on-demand instances.
//!   A struct usually provides a single target type (but this is not enforced).
//! * The [Resolve] trait indicates that a struct can be used to build an instance of [Provide].
//!   It provides the building-block of the injection mechanism.
//!
//! The resolver mechanism enables to create the injection rules, but is not exposed directly to consumers.
//!
//! * The [ProviderMap] trait describes a collection of providers (in practice using a type map).
//!   It is the base trait for the dependency injection but has no compile time guarantees.

use std::sync::Arc;
use thiserror::Error;

use crate::inject::Callable;

/// Provide an instance of a given type
///
/// This trait allows to use a uniform API for both
/// shared components (the provider holds the singleton)
/// and on-demand instances (the provider is a factory).
pub trait Provide<T>: Send + Sync {
    fn provide(&self) -> T;
}

/// Shared trait object implementing [Provide]
pub type Provider<T> = Arc<dyn Provide<T>>;

/// Generic collection of providers
///
/// This trait represents a map associating a type to a provider for this type.
/// It relies on external resolver to create these resolvers.
pub trait ProviderMap: Sized {
    /// Obtain a provider for the target type.
    ///
    /// If the provider is already stored in the map, returns the existing provider,
    ///  otherwise use the resolver module to build a new provider and store it in the map.
    fn resolve_with<R, T: ResolvedBy<R> + 'static>(&mut self, resolver: &R) -> &Provider<T>;

    /// Call a function after injecting its parameter(s).
    fn inject_and_call<R, F, I, O>(&mut self, resolver: &R, f: F) -> O
    where
        I: Injectable<R>,
        F: Callable<I, O>,
    {
        f.call(I::inject(resolver, self))
    }

    /// Obtain a provider for the parameter(s) of a callable function
    fn inject_provider<R, F, I, O>(&mut self, _resolver: &R, _f: F) -> Provider<I>
    where
        I: Injectable<R>,
        F: Callable<I, O>,
    {
        I::provide(_resolver, self)
    }
}

/// Obtain a provider for the target type.
pub trait Resolve<T>: Sized {
    /// Construct a provider for the target type.
    ///
    /// This function should not be called directly but will be triggered by the injector when needed
    fn build_provider(&self, injector: &mut impl ProviderMap) -> Provider<T>;
}

/// Mark a type as resolvable by a given resolver
pub trait ResolvedBy<R> {
    fn build_provider(resolver: &R, injector: &mut impl ProviderMap) -> Provider<Self>;
}

impl<T, R: Resolve<T>> ResolvedBy<R> for T {
    fn build_provider(resolver: &R, injector: &mut impl ProviderMap) -> Provider<Self> {
        resolver.build_provider(injector)
    }
}

/// Errors triggered during the autowiring process
#[derive(Error, Debug)]
pub enum WiringError {
    #[error("Cyclic dependencies: trying to start resolving in an open slot")]
    CyclicResolution,
    #[error("Consistency error: trying to replace an existing dependency")]
    AlreadyResolved,
}

/// Mark a derived type as resolvable by a given resolver
///
/// This trait is implemented for tuples of resolved types
pub trait Injectable<R>: Sized {
    fn inject(resolver: &R, injector: &mut impl ProviderMap) -> Self;
    fn provide(resolver: &R, injector: &mut impl ProviderMap) -> Provider<Self>;
}

/// Generic clone-based provider
pub struct SingletonProvider<T>(T);

impl<T> SingletonProvider<T> {
    pub fn build(data: T) -> Arc<Self> {
        Arc::new(SingletonProvider(data))
    }
}

impl<T: Clone + Send + Sync> Provide<T> for SingletonProvider<T> {
    fn provide(&self) -> T {
        self.0.clone()
    }
}

/// Generic provider for single-use instances based on a callable constructor
pub struct InstanceProvider<I, F> {
    pub provider: Provider<I>,
    pub constructor: F,
}

impl<I, F> InstanceProvider<I, F> {
    pub fn new(provider: Provider<I>, constructor: F) -> Self {
        Self {
            provider,
            constructor,
        }
    }
}

impl<I, T, F: Callable<I, T> + Send + Sync> Provide<T> for InstanceProvider<I, F> {
    fn provide(&self) -> T {
        self.constructor.call(self.provider.provide())
    }
}

/// Declare that a field of the parent type is a resolver submodules.
///
/// This will import and delegate all resolution rules of the submodule using a blanket implementation.
/// Note that conflicts can appear if a type is resolved by both the submodule and the parent (directly or through another submodule).
#[macro_export]
macro_rules! resolve_delegated {
    ($Proxy: ty $(, $Resolver:ty => $field: ident)+ ) => {
        $(
        impl<T: $crate::resolve::ResolvedBy<$Resolver>> $crate::resolve::Resolve<T> for $Proxy {
            fn build_provider(&self, injector: &mut impl $crate::resolve::ProviderMap) -> $crate::resolve::Provider<T> {
                T::build_provider(&self.$field, injector)
            }
        }
    )+
    };
}

/// Declare that our resolver module can provide a shared singleton of the selected type.
///
/// This macro provides a generic implementation of ```Resolve<Arc<$Type>>``` for ```$Resolver```.
/// The singleton instance is obtained by calling the ```$constructor``` function.
/// All parameters of this function must be injectable using the same resolver type.
#[macro_export]
macro_rules! resolve_singleton {
    ($Resolver:ty $(, $Type:ty => $constructor: expr)+) => {
        $(
        impl $crate::resolve::Resolve<Arc<$Type>> for $Resolver {
            fn build_provider(&self, injector: &mut impl $crate::resolve::ProviderMap) -> $crate::resolve::Provider<Arc<$Type>> {
                let singleton: Arc<$Type> = Arc::new(injector.inject_and_call(self, &$constructor));
                $crate::resolve::SingletonProvider::build(singleton)
            }
        }
        )+
    };
}

/// Declare that our resolver module can create on-demand instances of the selected type.
///
/// If the selected type is a raw (unboxed) concrete type, only a constructor function is required.
///
/// For trait objects or smart pointers, we also need to specify the boxing type (Box, Rc, Arc) as well as
/// the concrete type to generate a wrapper between the concrete type and the target. This wrapper uses a
/// struct named ```{$Resover}Wrapper``` that must be created beforehand as it must be local to be allowed
///  to add impl and we want to share a single generic struct as much as possible.
#[macro_export]
macro_rules! resolve_instance {
    ($Resolver:ty $(, $Type:ty => $constructor: expr)+) => {
        $(
        impl $crate::resolve::Resolve<$Type> for $Resolver {
            fn build_provider(&self, injector: &mut impl $crate::resolve::ProviderMap) -> $crate::resolve::Provider<$Type> {
                let prv = injector.inject_provider(self, $constructor);
                let factory = $crate::resolve::InstanceProvider::new(prv, $constructor);
                Arc::new(factory)
            }
        }
        )+
    };
    ($Resolver:ty $(, $bx: ident : $Type:ty => $Concrete: ty : $constructor: expr)+) => {
        $(
        impl<T: $crate::resolve::Provide<$Concrete>> $crate::resolve::Provide<$bx<$Type>> for paste::paste! { [< $Resolver Wrapper >]<T> } {
            fn provide(&self) -> $bx<$Type> {
                let concrete: $Concrete = self.0.provide();
                $bx::new(concrete)
            }
        }
        impl $crate::resolve::Resolve<$bx<$Type>> for $Resolver {
            fn build_provider(&self, injector: &mut impl $crate::resolve::ProviderMap) -> $crate::resolve::Provider<$bx<$Type>> {
                let prv = injector.inject_provider(self, $constructor);
                let factory = $crate::resolve::InstanceProvider::new(prv, $constructor);
                Arc::new(paste::paste! { [< $Resolver Wrapper >] }(factory))
            }
        }
        )+
    };
}
