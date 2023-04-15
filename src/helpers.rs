use std::sync::Arc;

use crate::Provide;

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

/// Declare that the proxy type can act as proxy-resolver for the resolver type
#[macro_export]
macro_rules! resolve_proxy {
    ($proxy: ty, $resolver:ty, $field: ident) => {
        impl<T: ResolvedBy<$resolver>> Resolve<T> for $proxy {
            fn build_provider(
                &self,
                injector: &mut impl ProviderMap,
            ) -> Result<Provider<T>, WiringError> {
                T::build_provider(&self.$field, injector)
            }
        }
    };
}

/// Derive ```Resolve<Arc<$target_type>>``` for ```$resolver_type```.
///
/// This macro will wrap an instance of the ```$concrete_type``` type in a ```Arc<$target_type>```.
///
/// * In absence of argument, it relies on the ```$concrete_type::default``` constructor.
/// * When additional arguments are available it will provide them in order to a ```$concrete_type::new``` function
#[macro_export]
macro_rules! resolve_singleton {
    ($target_type: ty, $concrete_type: ty, $resolver_type:ty) => {
        impl Resolve<Arc<$target_type>> for $resolver_type {
            fn build_provider(&self, _injector: &mut impl ProviderMap) -> Result<Provider<Arc<$target_type>>, WiringError> {
                let singleton: Arc<$target_type> = Arc::new(<$concrete_type>::default());
                Ok(SingletonProvider::build(singleton))
            }
        }
    };
    ($target_type: ty, $concrete_type: ty, $resolver_type:ty, $($args:tt)*) => {
        impl Resolve<Arc<$target_type>> for $resolver_type {
            fn build_provider(&self, injector: &mut impl ProviderMap) -> Result<Provider<Arc<$target_type>>, WiringError> {
                let singleton: Arc<$target_type> = Arc::new(<$concrete_type>::new(inject_tt!(injector, self, $($args)*)));
                Ok(SingletonProvider::build(singleton))
            }
        }
    };
}

/// Token Tree muncher for the extra arguments of the ```resolve_singleton``` macro
#[doc(hidden)]
#[macro_export]
macro_rules! inject_tt {
    ($injector: ident, $resolver: ident, $type: ty) => {
        $injector.inject_with($resolver)?
    };

    ($injector: ident, $resolver: ident, $type: ty, $($b:tt)*) => {
        injector.inject_with(self)?, inject_tt!($injector, $resolver, $($b)*)
    };
}
