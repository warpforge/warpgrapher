use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn wg_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input: ItemFn = parse_macro_input!(item);

    let name = &input.sig.ident;
    let name_gremlin = format_ident!("{}{}", name, "_gremlin");
    let name_cypher = format_ident!("{}{}", name, "_cypher");

    let gen = quote! {
        #[cfg(feature = "gremlin")]
        #[tokio::test]
        async fn #name_gremlin() {
            setup::init();
            setup::clear_db().await;

            let client = setup::gremlin_test_client("./tests/fixtures/minimal.yml").await;
            #name(client).await;
        }

        #[cfg(feature = "cypher")]
        #[tokio::test]
        async fn #name_cypher() {
            setup::init();
            setup::clear_db().await;

            let client = setup::cypher_test_client("./tests/fixtures/minimal.yml").await;
            #name(client).await;
        }

        #input
    };

    gen.into()
}
