//! Experimental compile-time dependency injection crate with composable resolution rules and modules.
//!
//! # Simple use case
//!
//! ```
//! # use std::sync::Arc;
//! # use hanami::*;
//! // Define struct, traits and implementors
//! #[derive(Default)]
//! struct MySharedData;
//!
//! trait MyTrait: Send + Sync {
//!     fn cheers(&self);
//! }
//!
//! #[derive(Default)]
//! struct MyImpl;
//!
//! impl MyTrait for MyImpl {
//!   fn cheers(&self) {
//!     println!("Hello world");
//!   }
//! }
//!
//!
//! // Define a resolver module and resolution rules
//! struct MyResolver;
//!
//! // Simple singleton of an explicit struct implementing Default
//! resolve_singleton!(MyResolver, MySharedData);
//!
//! // Singleton of a trait object
//! resolve_singleton!(MyResolver, dyn MyTrait: MyImpl);
//!
//!
//! # fn main() -> Result<(), WiringError> {
//! // Create and use an injector using our resolver module
//! let injector = Hanami::new(MyResolver);
//! let a: Arc<dyn MyTrait> = injector.inject()?;
//! a.cheers();
//! # Ok(())
//! # }
//! ```
//!
//! # Mechanism
//!
//! The injection mechanism combines a generic singleton map based on the [std::any::Any] trait for runtime reflection
//! with specialised traits to support each target types, enabling to ensure that a type is injectable
//! at compile-time. These traits are implementd in the [Hanami] struct.
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
//! * The [Hanami] struct combines a [ProviderMap] with a resolver module.
//!   It implements the [Inject] trait for all types resolved by the resolver module.
//!   This provides additional compile-time guarantees on the injectable types, controlled by
//!   implementations of [Resolve] associated to the resolver module.
//!
//! # Composition
//!
//! The resolution rules can be defined using several resolver modules and composed using a proxy resolver.
//! A proxy resolver contains another resolver module and will forward the resolution of its associated types.
//! See the [resolve_proxy] macro to declare proxy resolution rules.
//!
//! # Cyclical dependencies
//!
//! Cyclical dependencies between injected types trigger an error at runtime (without infinite looping).
//! As resolution rules are independent, they can not be detected at compile time (this is the case in
//! [shaku](https://crates.io/crates/shaku) when using a single macro to define all resolution rules at once).
//!
//! # Override
//!
//! The user can override the provider for a given target type **before the first runtime-resolution of this type**.
//! This allows to set a mock or an alternative implementation at runtime.
//! See the [Inject::set_provider] function.

mod helpers;
mod inject;
mod resolve;

pub use helpers::SingletonProvider;
pub use inject::{Hanami, Inject};
pub use resolve::{Provide, Provider, ProviderMap, Resolve, ResolvedBy, WiringError};

#[cfg(test)]
mod tests;
