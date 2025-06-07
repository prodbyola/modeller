# How MODELLER works
This flow is subjected to future changes depending benchmarks and performance.

### Model Parsing and Modeller Initialization
- `modeller::define_model!` macro accepts struct definitions as arguments.
- It passes the models to `parser::parse_models!` which parses the models into `definitions::core::DefinitionStream`.
- `DefinitionStream` takes the models as a vector of `syn::ItemStruct` as `items` and then provides a `raw()` method. `raw()` maps the `ItemStruct` vector into `definitions::core::ModelDefinition` vector, which is then encoded and returned as `&[u8]`.
NOTE: Earlier, we were returning `&[String]` but returning `&[u8]` reduced our test output from 997 to 132 bytes!
- `parse_models!` uses DefinitionStream's `items` to regenerate rust structs from original models passed as arguments. Then it converts DefinitionStream into a function `modeller_definition_streams` which returns `&[u8]` from `models()` method. We pass `def_streams_list` into the `TokenStream` to make it available to `parse_models`'s caller.
- Finally, `modeller::define_model!` provides `get_modeller` function which takes `&[u8]` argument and uses it to create a new `Modeller` instance.

### Generating and Running Migrations
