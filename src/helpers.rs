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

/// Derive ```Resolve<Arc<$singleton_type>>``` for ```$resolver_type```.
///
/// This macro will wrap an instance of the ```$concrete_type``` type in a ```Arc<$singleton_type>```.
/// If the injectable type is a struct implementing default, then no extra information is needed.
/// Otherwise, the macro allows to associate a trait to a concrete type, select the name of the constructor function
/// and specify the list of arguments, which must all be resolved by the ```$resolver_type```.
#[macro_export]
macro_rules! resolve_singleton {
    ($resolver_type:ty, $singleton_type: ty) => {
        resolve_singleton!($resolver_type, $singleton_type : $singleton_type);
    };
    ($resolver_type:ty, $singleton_type: ty: $concrete_type: ty) => {
        resolve_singleton!($resolver_type, $singleton_type : $concrete_type, default);
    };
    ($resolver_type:ty, $singleton_type:ty : $concrete_type: ty, $constructor: ident) => {
        impl Resolve<Arc<$singleton_type>> for $resolver_type {
            fn build_provider(&self, _injector: &mut impl ProviderMap) -> Result<Provider<Arc<$singleton_type>>, WiringError> {
                let singleton: Arc<$singleton_type> = inject_into_provider!($concrete_type, _injector, self, $constructor);
                Ok(SingletonProvider::build(singleton))
            }
        }
    };
    ($resolver_type:ty, $singleton_type: ty, $constructor: ident, $($args:tt),*) => {
        resolve_singleton!($resolver_type, $singleton_type : $singleton_type, $constructor, $($args,)*);
    };
    ($resolver_type:ty, $singleton_type:ty : $concrete_type: ty, $constructor: ident, $($args:tt)*) => {
        impl Resolve<Arc<$singleton_type>> for $resolver_type {
            fn build_provider(&self, _injector: &mut impl ProviderMap) -> Result<Provider<Arc<$singleton_type>>, WiringError> {
                let singleton: Arc<$singleton_type> = inject_into_provider!($concrete_type, _injector, self, $constructor, $($args)*);
                Ok(SingletonProvider::build(singleton))
            }
        }
    };
}

/// Helper macro to construct the providers and inject their parameters
#[doc(hidden)]
#[macro_export]
macro_rules! inject_into_provider {
    ($concrete_type: ty, $injector: ident, $resolver: ident, $constructor: ident) => {
        Arc::new(<$concrete_type>::$constructor())
    };
    ($concrete_type: ty, $injector: ident, $resolver: ident, $constructor: ident, $($args:tt)*) => {
        Arc::new(<$concrete_type>::$constructor(inject_tt!($injector, $resolver, $($args)*)))
    };
}

/// Token Tree muncher for the extra arguments of the ```resolve_singleton``` macro
#[doc(hidden)]
#[macro_export]
macro_rules! inject_tt {
    ($injector: ident, $resolver: ident, $type: ty) => {
        $injector.inject_with($resolver)?
    };

    ($type: ty, $($b:tt)*) => {
        _injector.inject_with(self)?, inject_tt!($($b)*)
    };
}

/// Token Tree muncher for the extra arguments of the ```resolve_provider``` macro
#[doc(hidden)]
#[macro_export]
macro_rules! resolve_tt {
    ($injector: ident, $resolver: ident, $name: ident: $type: ty) => {
        $injector.resolve_with($resolver)?
    };

    ($injector: ident, $resolver: ident, $name: ident: $type: ty, $($b:tt)*) => {
        injector.resolve_with(self)?, resolve_tt!($injector, $resolver, $($b)*)
    };
}

#[macro_export]
macro_rules! resolve_provider {
    ($resolver_type:ty, $singleton_type: ty, $provider_type: ident) => {
        resolve_provider!($resolver_type, $singleton_type: $singleton_type, $provider_type);
    };
    ($resolver_type:ty, $singleton_type: ty : $concrete_type: ty, $provider_type: ident) => {
        resolve_provider!($resolver_type, $singleton_type: $concrete_type, $provider_type, default);
    };

    ($resolver_type:ty, $singleton_type: ty, $provider_type: ident, $constructor: ident) => {
        resolve_provider!($resolver_type, $singleton_type: $singleton_type, $provider_type, $constructor);
    };

    ($resolver_type:ty, $singleton_type: ty : $concrete_type: ty, $provider_type: ident, $constructor: ident) => {
        #[derive(Default)]
        struct $provider_type;

        impl Provide<Arc<$singleton_type>> for $provider_type {
            fn provide(&self) -> Arc<$singleton_type> {
                let instance: Arc<$singleton_type> = Arc::new(<$concrete_type>::$constructor());
                instance
            }
        }

        impl Resolve<Arc<$singleton_type>> for $resolver_type {
            fn build_provider(&self, _injector: &mut impl ProviderMap) -> Result<Provider<Arc<$singleton_type>>, WiringError> {
                let provider: $provider_type = <$provider_type>::default();
                Ok(Arc::new(provider))
            }
        }
    };
    ($resolver_type:ty, $singleton_type: ty : $concrete_type: ty, $provider_type: ident, $constructor: ident, $($args:tt)*) => {

        // TODO: create the provider struct!

        impl Resolve<Arc<$singleton_type>> for $resolver_type {
            fn build_provider(&self, injector: &mut impl ProviderMap) -> Result<Provider<Arc<$singleton_type>>, WiringError> {
                let singleton: Arc<$singleton_type> = Arc::new(<$concrete_type>::$constructor(resolve_tt!(injector, self, $($args)*)));
                Ok(SingletonProvider::build(singleton))
            }
        }
    };
}
