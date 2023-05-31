use std::sync::Arc;

use crate::{inject::Callable, Provide, Provider};

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
pub struct FactoryProvider<I, F> {
    pub provider: Provider<I>,
    pub constructor: F,
}

impl<I, F> FactoryProvider<I, F> {
    pub fn new(provider: Provider<I>, constructor: F) -> Self {
        Self {
            provider,
            constructor,
        }
    }
}

impl<I, T, F: Callable<I, T> + Send + Sync> Provide<T> for FactoryProvider<I, F> {
    fn provide(&self) -> T {
        self.constructor.call(self.provider.provide())
    }
}

/// Declare that the proxy type can act as proxy-resolver for the resolver type
#[macro_export]
macro_rules! resolve_proxy {
    ($Proxy: ty $(, $Resolver:ty => $field: ident)+ ) => {
        $(
        impl<T: ResolvedBy<$Resolver>> Resolve<T> for $Proxy {
            fn build_provider(&self, injector: &mut impl ProviderMap) -> Provider<T> {
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
        impl Resolve<Arc<$Type>> for $Resolver {
            fn build_provider(&self, injector: &mut impl ProviderMap) -> Provider<Arc<$Type>> {
                let singleton: Arc<$Type> = Arc::new(injector.inject_and_call(self, &$constructor));
                SingletonProvider::build(singleton)
            }
        }
        )+
    };
}

/// Declare that our resolver module can create on-demand instances of the selected type.
///
/// This relies on the definition of a struct ```{$Resover}Wrapper```used as an intermediate factory for the selected type.
/// This struct is then used to implement ```Resolve<$bx<$Type>>``` for ```$Resolver```.
#[macro_export]
macro_rules! resolve_instance {
    ($Resolver:ty $(, $bx: ident : $Type:ty => $Concrete: ty : $constructor: expr)+) => {
        $(
        impl<T: Provide<$Concrete>> Provide<$bx<$Type>> for paste::paste! { [< $Resolver Wrapper >]<T> } {
            fn provide(&self) -> $bx<$Type> {
                let concrete: $Concrete = self.0.provide();
                $bx::new(concrete)
            }
        }
        impl Resolve<$bx<$Type>> for $Resolver {
            fn build_provider(&self, injector: &mut impl ProviderMap) -> Provider<$bx<$Type>> {
                let prv = injector.inject_provider(self, $constructor);
                let factory = FactoryProvider::new(prv, $constructor);
                Arc::new(paste::paste! { [< $Resolver Wrapper >] }(factory))
            }
        }
        )+
    };
}

/// Declare that our resolver module can create on-demand instances of the selected type.
///
/// This relies on the definition of a struct ```$provider_type```used as an intermediate factory for the selected type.\
/// This struct is then used to implement ```Resolve<$instance_type>``` for ```$resolver_type```.
#[macro_export]
macro_rules! resolve_raw_instance {
    ($resolver_type:ty, $instance_type: ty) => {
        resolve_raw_instance!($resolver_type, $instance_type, default);
    };
    ($resolver_type:ty, $instance_type: ty : $provider_type: ident) => {
        resolve_raw_instance!($resolver_type, $instance_type : $provider_type, default);
    };
    ($resolver_type:ty, $instance_type: ty, $constructor: ident $(, $arg_name: ident : $arg_type: ty)*) => {
        paste::paste!{
            resolve_raw_instance!($resolver_type, $instance_type :  [< $instance_type Factory >], $constructor $(, $arg_name : $arg_type )*);
        }
    };
    ($resolver_type:ty, $instance_type: ty : $provider_type: ident, $constructor: ident $(, $arg_name: ident : $arg_type: ty)*) => {
        struct $provider_type { $( $arg_name: Arc<dyn Provide<$arg_type>>, )* }

        impl Provide<$instance_type> for $provider_type {
            fn provide(&self) -> $instance_type {
                <$instance_type>::$constructor( $( self.$arg_name.provide(), )* )
            }
        }

        impl Resolve<$instance_type> for $resolver_type {
            fn build_provider(&self, _injector: &mut impl ProviderMap) -> Provider<$instance_type> {
                Arc::new( $provider_type {
                    $( $arg_name: _injector.resolve_with::<$arg_type>(self).clone(), )*
                } )
            }
        }
    };
}
