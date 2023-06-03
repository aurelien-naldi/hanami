# hanami

Experimental compile-time dependency injection crate with composable resolution rules and modules.


The resolution rules are declared using macros that define (for a given resolution module) a map associating
resolvable types to the constructors used to create instances. All parameters of the constructor must be
resolvable types. Note that constructors are currently limited to 10 parameters.
These macros can be used multiple times on the same resolver module for different target types.
Resolution rules can also be composed using submodules. The parent module must contain instances of the submodules
and delegates the resolution of some of its associated types to the relevant submodule.


```rust
// Define traits and implementors
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
hanami::resolve_singleton!(MyResolver,  dyn MyTrait => MyImpl::default);


// Create and use an injector based on the resolver module
let injector = hanami::Hanami::new(MyResolver);
let mt: Arc<dyn MyTrait> = injector.inject();
mt.cheers();
```

## Override

The user can override the provider for a given target type **before the first runtime-resolution of this type**.
This allows for example to set a mock or an alternative implementation at runtime.
See the [Hanami::set_provider] function.

## Panic on cyclical dependencies

Cyclical dependencies between injected types avoid infinite loops but trigger a panic at runtime.
As resolution rules are independent, they can not be detected at compile time (this is the case in
[shaku](https://crates.io/crates/shaku) when using a single macro to define all resolution rules at once).

