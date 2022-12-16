# hanami

Experimental dependency injection crate, inspired from shaku and spring.


# Mechanism

This crate defines a ```Registry<M>``` struct to store singletons of any type (using Any for type instrospection at runtime).

as well as a ```Factory<T>``` trait to register builder function for some types.



