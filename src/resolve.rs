use std::{ops::Deref, sync::Arc};
use thiserror::Error;

/// Provide an instance of a given type
///
/// This trait allows to use a uniform API for both
/// shared components (singletons) and
/// factories (on-demand instances).
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
    fn resolve_with<T: 'static>(
        &mut self,
        resolver: &impl Resolve<T>,
    ) -> Result<&Provider<T>, WiringError>;

    fn inject_with<T: 'static>(&mut self, resolver: &impl Resolve<T>) -> Result<T, WiringError> {
        self.resolve_with::<T>(resolver)
            .map(|p| p.deref().provide())
    }
}

/// Obtain a provider for the target type.
pub trait Resolve<T>: Sized {
    /// Construct a provider for the target type.
    ///
    /// This function should not be called directly but will be triggered by the injector when needed
    fn build_provider(&self, injector: &mut impl ProviderMap) -> Result<Provider<T>, WiringError>;
}

/// Mark a type as resolvable by a given resolver
pub trait ResolvedBy<R> {
    fn build_provider(
        resolver: &R,
        injector: &mut impl ProviderMap,
    ) -> Result<Provider<Self>, WiringError>;
}

impl<T, R: Resolve<T>> ResolvedBy<R> for T {
    fn build_provider(
        resolver: &R,
        injector: &mut impl ProviderMap,
    ) -> Result<Provider<Self>, WiringError> {
        resolver.build_provider(injector)
    }
}

/// Errors triggered during the autowiring process
#[derive(Error, Debug)]
pub enum WiringError {
    #[error("A singleton is missing from the read-only store, did you call resolve at startup?")]
    SingletonIsMissing,
    #[error("Cyclic dependencies: trying to start resolving in an open slot")]
    CyclicResolution,
    #[error("Consistency error: trying to replace an existing dependency")]
    AlreadyResolved,
}
