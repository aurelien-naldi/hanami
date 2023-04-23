# hanami

Experimental compile-time dependency injection crate with composable resolution rules and modules.

## Simple use case

```rust
#
// Define struct, traits and implementors
#[derive(Default)]
struct MySharedData;

trait MyTrait: Send + Sync {
    fn cheers(&self);
}

#[derive(Default)]
struct MyImpl;

impl MyTrait for MyImpl {
  fn cheers(&self) {
    println!("Hello world");
  }
}


// Define a resolver module and resolution rules
struct MyResolver;

// Simple singleton of an explicit struct implementing Default
resolve_singleton!(MyResolver, MySharedData);

// Singleton of a trait object
resolve_singleton!(MyResolver, dyn MyTrait: MyImpl);


// Create and use an injector using our resolver module
let injector = Hanami::new(MyResolver);
let a: Arc<dyn MyTrait> = injector.inject()?;
a.cheers();
```

## Mechanism

The injection mechanism combines a generic singleton map based on the ```Any``` trait for runtime reflection
with specialised traits to support each target types, enabling to ensure that a type is injectable
at compile-time. These traits are implementd in the ```Hanami``` struct.

 To inject a type ```T```, we first need to define a provider and a resolver for this type.

* The ```Provide<T>``` trait indicates that a struct can provide an instance of the target type.
  The provider can be either a singleton of the target type or a factory for on-demand instances.
  A struct usually provides a single target type (but this is not enforced).
* The ```Resolve<T>``` trait indicates that a struct can be used to build an instance of ```Provide<T>```.
  It provides the building-block of the injection mechanism.

The resolver mechanism enables to create the injection rules, but is not exposed directly to consumers.

* The ```ProviderMap``` trait describes a collection of providers (in practice using a type map).
  It is the base trait for the dependency injection but has no compile time guarantees.
* The ```Hanami<R>``` struct combines a ```ProviderMap``` with the resolver module ```R```.
  It implements the ```Inject<T>``` trait for all types ```T``` resolved by ```R```.
  This provides additional compile-time guarantees on the injectable types, controlled by
  implementations of ```Resolve<T>``` associated to ```R```.
