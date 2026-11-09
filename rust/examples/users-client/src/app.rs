use crate::client::CLIENT;
use bloom_client::async_error_to_console;
use bloom_core::{Element, component, use_spawner};
use bloom_html::{Form, HtmlNode, prelude::*};
use bloom_rsx::rsx;
use bloom_toapi::{Error, use_query};
use wasm_bindgen::JsError;

#[component]
pub(super) async fn UsersApp() -> Result<Element<HtmlNode, Error>, Error> {
    let users = use_query(CLIENT.with(|client| client.users.get())).await?;
    let spawner = use_spawner();

    Ok(rsx!(
        <div>
            <div>"Users"</div>
            <ul>
              {users.iter()
                  .map(|user| rsx!(
                      <li key={user.id}>
                        {user.name.clone()}
                      </li>
                  )).collect::<Vec<_>>()}
            </ul>
            <Form action={move |form_data| {
                let _  = spawner.spawn(
                    async_error_to_console(async move || {
                        let name = form_data.get("name").ok_or(JsError::new("No Name provided"))?.as_string().ok_or(JsError::new("Name is not a string"))?;
                        CLIENT.with(|client| {
                            client.users.post()
                                .name(name)
                                .role(crate::client::Role::Member)
                        }).await.map_err(Into::<JsError>::into)?;
                        Ok(())
                    })
                );
            }}>
              <input name="name" />
              <button>"Submit"</button>
            </Form>
        </div>
    ))
}
