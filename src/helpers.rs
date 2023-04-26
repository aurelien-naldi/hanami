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

/// Declare that our resolver module can provide a shared singleton of the selected type.
///
/// This macro provides a generic implementation of ```Resolve<Arc<$singleton_type>>``` for ```$resolver_type```.
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
    ($resolver_type:ty, $singleton_type: ty, $constructor: ident $(, $param_type:ty)*) => {
        resolve_singleton!($resolver_type, $singleton_type : $singleton_type, $constructor $(, $param_type)*);
    };
    ($resolver_type:ty, $singleton_type:ty : $concrete_type: ty, $constructor: ident $(, $param_type:ty)*) => {
        impl Resolve<Arc<$singleton_type>> for $resolver_type {
            fn build_provider(&self, _injector: &mut impl ProviderMap) -> Result<Provider<Arc<$singleton_type>>, WiringError> {
                let singleton: Arc<$singleton_type> = Arc::new(<$concrete_type>::$constructor( $(_injector.inject_with::<$param_type>(self)?, )* ));
                Ok(SingletonProvider::build(singleton))
            }
        }
    };
}

/// Declare that our resolver module can create on-demand instances of the selected type.
///
/// This relies on the definition of a struct ```$provider_type```used as an intermediate factory for the selected type.\
/// This struct is then used to implement ```Resolve<Arc<$instance_type>>``` for ```$resolver_type```.
#[macro_export]
macro_rules! resolve_instance {
    ($resolver_type:ty, $instance_type: ty, $provider_type: ident) => {
        resolve_instance!($resolver_type, $instance_type: $instance_type, $provider_type);
    };
    ($resolver_type:ty, $instance_type: ty : $concrete_type: ty, $provider_type: ident) => {
        resolve_instance!($resolver_type, $instance_type: $concrete_type, $provider_type, default);
    };
    ($resolver_type:ty, $instance_type: ty : $bx: ident : $concrete_type: ty, $provider_type: ident) => {
        resolve_instance!($resolver_type, $instance_type: $bx : $concrete_type, $provider_type, default);
    };
    ($resolver_type:ty, $instance_type: ty, $provider_type: ident, $constructor: ident $(, $arg_name: ident : $arg_type: ty)*) => {
        struct $provider_type { $( $arg_name: Arc<dyn Provide<$arg_type>>, )* }

        impl Provide<$instance_type> for $provider_type {
            fn provide(&self) -> $instance_type {
                <$instance_type>::$constructor( $( self.$arg_name.provide(), )* )
            }
        }

        impl Resolve<$instance_type> for $resolver_type {
            fn build_provider(&self, _injector: &mut impl ProviderMap) -> Result<Provider<$instance_type>, WiringError> {
                Ok(Arc::new( $provider_type {
                    $( $arg_name: _injector.resolve_with::<$arg_type>(self)?.clone(), )*
                } ))
            }
        }
    };
    ($resolver_type:ty, $instance_type: ty : $concrete_type: ty, $provider_type: ident, $constructor: ident $(, $arg_name: ident : $arg_type: ty)*) => {
        resolve_instance!($resolver_type, $instance_type : Box : $concrete_type, $provider_type, $constructor $(, $arg_name : $arg_type )*);
    };
    ($resolver_type:ty, $instance_type: ty : $bx: ident : $concrete_type: ty, $provider_type: ident, $constructor: ident $(, $arg_name: ident : $arg_type: ty)*) => {
        struct $provider_type { $( $arg_name: Arc<dyn Provide<$arg_type>>, )* }

        impl Provide<$bx<$instance_type>> for $provider_type {
            fn provide(&self) -> $bx<$instance_type> {
                $bx::new(<$concrete_type>::$constructor( $( self.$arg_name.provide(), )* ))
            }
        }

        impl Resolve<$bx<$instance_type>> for $resolver_type {
            fn build_provider(&self, _injector: &mut impl ProviderMap) -> Result<Provider<$bx<$instance_type>>, WiringError> {
                Ok(Arc::new( $provider_type {
                    $( $arg_name: _injector.resolve_with::<$arg_type>(self)?.clone(), )*
                } ))
            }
        }
    };
}
