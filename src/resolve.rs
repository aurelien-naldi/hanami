use std::sync::Arc;
use thiserror::Error;

use crate::{inject::Callable, Injectable};

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
    fn resolve_with<T: 'static>(&mut self, resolver: &impl Resolve<T>) -> &Provider<T>;

    fn inject_with<T: 'static>(&mut self, resolver: &impl Resolve<T>) -> T {
        self.resolve_with::<T>(resolver).provide()
    }

    fn resolved_with<R, T: ResolvedBy<R> + 'static>(&mut self, resolver: &R) -> &Provider<T>;

    fn injected_with<R, T: ResolvedBy<R> + 'static>(&mut self, resolver: &R) -> T {
        self.resolved_with::<R, T>(resolver).provide()
    }

    fn inject_and_call<R, F, I, O>(&mut self, resolver: &R, f: F) -> O
    where
        I: Injectable<R>,
        F: Callable<I, O>,
    {
        f.call(I::inject(resolver, self))
    }

    fn inject_provider<R, F, I, O>(&mut self, _resolver: &R, _f: F) -> Provider<I>
    where
        I: Injectable<R>,
        F: Callable<I, O>,
    {
        let provider: Provider<I> = I::provide(_resolver, self);
        provider
        // Ok(f.call(I::inject(resolver, self)?))
        //        unimplemented!();
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
