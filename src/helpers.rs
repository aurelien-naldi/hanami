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
        impl Resolve<$Type> for $Resolver {
            fn build_provider(&self, injector: &mut impl ProviderMap) -> Provider<$Type> {
                let prv = injector.inject_provider(self, $constructor);
                let factory = FactoryProvider::new(prv, $constructor);
                Arc::new(factory)
            }
        }
        )+
    };
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
