use proc_macro::TokenStream;
use quote::{quote, ToTokens};

#[proc_macro_attribute]
pub fn message_handler(_: TokenStream, item: TokenStream) -> TokenStream {
    let ast: syn::ItemFn = syn::parse(item).unwrap();
    let func_ident = ast.sig.ident.clone();

    let gen = quote! {
        mod m {
            extern "C" {
                pub fn get_event_body_length() -> i32;
                pub fn get_event_body(p: *mut u8) -> i32;
            }
        }

        fn message_from_subscription() -> Option<Message> {
            unsafe {
                let l = m::get_event_body_length();
                let mut event_body = Vec::<u8>::with_capacity(l as usize);
                let c = m::get_event_body(event_body.as_mut_ptr());
                assert!(c == l);
                event_body.set_len(c as usize);

                match serde_json::from_slice::<Message>(&event_body) {
                    Ok(e) => Some(e),
                    Err(_) => None,
                }
            }
        }

        #[no_mangle]
        #[tokio::main(flavor = "current_thread")]
        pub async fn on_message_received() {
            if let Some(m) = message_from_subscription() {
                #func_ident(m).await;
            }
        }
    };

    let ori_run_str = ast.to_token_stream().to_string();
    let x = gen.to_string() + &ori_run_str;
    x.parse().unwrap()
}

#[proc_macro_attribute]
pub fn application_command_handler(_: TokenStream, item: TokenStream) -> TokenStream {
    let ast: syn::ItemFn = syn::parse(item).unwrap();
    let func_ident = ast.sig.ident.clone();

    let gen = quote! {
        mod n {
            extern "C" {
                pub fn get_event_body_length() -> i32;
                pub fn get_event_body(p: *mut u8) -> i32;
            }
        }

        fn application_command_from_subscription() -> Option<ApplicationCommandInteraction> {
            unsafe {
                let l = n::get_event_body_length();
                let mut event_body = Vec::<u8>::with_capacity(l as usize);
                let c = n::get_event_body(event_body.as_mut_ptr());
                assert!(c == l);
                event_body.set_len(c as usize);

                match serde_json::from_slice::<ApplicationCommandInteraction>(&event_body) {
                    Ok(e) => Some(e),
                    Err(_) => None,
                }
            }
        }

        #[no_mangle]
        #[tokio::main(flavor = "current_thread")]
        pub async fn on_application_command_received() {
            if let Some(m) = application_command_from_subscription() {
                #func_ident(m).await;
            }
        }
    };

    let ori_run_str = ast.to_token_stream().to_string();
    let x = gen.to_string() + &ori_run_str;
    x.parse().unwrap()
}
